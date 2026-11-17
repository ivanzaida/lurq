//! The page of the engine's image node: pixels in a box, fitted to it as the node says, and the same pixels as the
//! background of any box. Its playground shows one of four pictures as its props say; its sections are the demo's
//! images (plain, sized, intrinsic, as backgrounds) and its moving ones (a GIF and a WebP, as images and as
//! backgrounds), with the decode the node does itself.
//!
//! `components::Image` is deprecated for apps: it is the engine's node `ui::Picture` draws with, and this page
//! shows that node. Every item that names it is under `#[allow(deprecated)]`.

use std::sync::{Arc, OnceLock};

// The engine's image node, which `ui::Picture` draws with: deprecated for apps, and what this page shows.
#[allow(deprecated)]
use lurq::components::Image;
use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Stack},
  core::Signal,
  images::{ImageData, ImageKey, ImageSource},
  layout::{Alignment, StackAlignment, text_style::FontWeight},
  node::{BackgroundSize, Element},
  ui::{Length, Skeleton},
};

use super::kit::samples;
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, fit, line, props_table, quiet, text},
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
const SIZES: [(&str, usize); 4] = [("width 160", 0), ("height 180", 1), ("200 × 200", 2), ("FILL × 220", 3)];
const RADII: [(&str, i32); 3] = [("0", 0), ("8", 8), ("24", 24)];
/// The side of the box a background is given where the size chosen names one side only.
const FREE_SIDE: f32 = 160.0;

/// The Rust of a size, as the playground's code says it: on an image, and on a box that has a background.
fn size_code(size: usize, background: bool) -> String {
  match (size, background) {
    (0, false) => ".width(160.0)".to_owned(),
    (1, false) => ".height(180.0)".to_owned(),
    (0, true) => format!(".size(160.0, {FREE_SIDE:.1})"),
    (1, true) => format!(".size({FREE_SIDE:.1}, 180.0)"),
    (2, _) => ".size(200.0, 200.0)".to_owned(),
    _ => ".size(Dimension::Pct(100.0), 220.0)".to_owned(),
  }
}

/// What the playground shows: the `picture`th picture, as an image or as the background of a box.
#[allow(deprecated)]
fn shown(picture: usize, fit: usize, size: usize, radius: i32, background: bool, border: bool) -> Element {
  let Some(pixels) = pixels(picture) else {
    return waiting(200.0, 200.0);
  };
  let fit = FITS[fit.min(FITS.len() - 1)].1;
  if background {
    let mut stage = Stack::new()
      .background(PaletteColor::Muted)
      .background_image(pixels)
      .background_size(fit)
      .child(label(ASSETS[picture].label).absolute_position(10.0, 10.0));
    stage = match size {
      0 => stage.size(160.0, FREE_SIDE),
      1 => stage.size(FREE_SIDE, 180.0),
      2 => stage.size(200.0, 200.0),
      _ => stage.size(FILL, 220.0),
    };
    if radius > 0 {
      stage = stage.rounded(radius as f32).clip();
    }
    if border {
      stage = stage.border(line());
    }
    return stage.into();
  }
  let mut image = Image::new(pixels).fit(fit);
  image = match size {
    0 => image.width(160.0),
    1 => image.height(180.0),
    2 => image.size(200.0, 200.0),
    _ => image.size(FILL, 220.0),
  };
  if radius > 0 {
    image = image.rounded(radius as f32).clip();
  }
  if border {
    image = image.border(line());
  }
  image.into()
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

/// A picture with what it is and its file under it.
fn captioned(shown: impl Into<Element>, asset: &Asset, what: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(6.0)
    .child(shown)
    .child(text(&format!("{what} {}", asset.label), 12.0, FontWeight::SemiBold))
    .child(quiet(asset.file, 11.0))
}

/// The four pictures, each in a box of 160 by 132 that the picture is stretched to: the node's default fit.
#[allow(deprecated)]
fn plain() -> lurq::components::Row {
  samples().with_children(ASSETS.iter().enumerate().map(|(index, asset)| {
    let shown = match pixels(index) {
      Some(pixels) => Image::new(pixels)
        .size(160.0, 132.0)
        .rounded(6.0)
        .clip()
        .border(line())
        .into(),
      None => waiting(160.0, 132.0),
    };
    captioned(Column::new().max_width(FILL).child(shown), asset, "Image")
  }))
}

/// The four pictures given a width alone: the height follows from the picture's proportions.
#[allow(deprecated)]
fn sized() -> lurq::components::Row {
  samples().with_children(ASSETS.iter().enumerate().map(|(index, asset)| {
    let shown = match pixels(index) {
      Some(pixels) => Image::new(pixels)
        .width(120.0)
        .rounded(6.0)
        .clip()
        .border(line())
        .into(),
      None => waiting(120.0, 120.0),
    };
    captioned(shown, asset, "width 120,")
  }))
}

/// The four pictures given no size: as many points as the picture has pixels.
#[allow(deprecated)]
fn intrinsic() -> lurq::components::Row {
  samples().with_children(ASSETS.iter().enumerate().map(|(index, asset)| {
    let shown = match pixels(index) {
      Some(pixels) => Image::new(pixels).rounded(6.0).clip().border(line()).into(),
      None => waiting(240.0, 240.0),
    };
    captioned(shown, asset, "Intrinsic")
  }))
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

/// The two pictures that move, as images as wide as they can be.
#[allow(deprecated)]
fn moving_images() -> lurq::components::Row {
  samples().with_children(MOVING.into_iter().map(|index| {
    let shown = match pixels(index) {
      Some(pixels) => Image::new(pixels)
        .size(FILL, 210.0)
        .cover()
        .rounded(8.0)
        .clip()
        .border(line())
        .into(),
      None => waiting(Length::Fill, 210.0),
    };
    captioned(
      Column::new().width(260.0).max_width(FILL).child(shown),
      &ASSETS[index],
      "Image",
    )
  }))
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

/// The four pictures as the app's resources, read and decoded by the engine: an image of each, and the GIF as
/// the background of a box.
#[cfg(not(target_arch = "wasm32"))]
#[allow(deprecated)]
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
    let image = Image::from_resource(&path)
      .size(FILL, 132.0)
      .cover()
      .rounded(6.0)
      .clip()
      .border(line());
    row = row.child(captioned(
      Column::new().width(160.0).max_width(FILL).child(image),
      asset,
      "from_resource",
    ));
  }
  if let Some(path) = resource(ASSETS[2].file) {
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
            .child(label("background").absolute_position(10.0, 10.0)),
        )
        .child(text("background_image(path)", 12.0, FontWeight::SemiBold))
        .child(quiet(ASSETS[2].file, 11.0)),
    );
  }
  row.into()
}

/// A browser's build reads no file: a line says what the desktop shows.
#[cfg(target_arch = "wasm32")]
fn resources() -> Element {
  text(
    "A browser's build reads no file of the app's: on a desktop the four pictures are read from their files here, \
     and the GIF is a box's background. The sections above show the same pictures.",
    13.0,
    FontWeight::Normal,
  )
  .color(PaletteColor::MutedForeground)
  .into()
}

/// Bytes that are no picture: what the node shows its error view for.
const BROKEN: &[u8] = b"not a picture";
/// The key the GIF is cached under when the node decodes it: given, so its 700 KB are not hashed at each render.
const GIF_KEY: u64 = 0x6E61_6D65_2E67_6966;

/// Pictures the node decodes itself, off the window's thread: a JPG, the first frame of the GIF, and bytes that are
/// no picture.
#[allow(deprecated)]
fn decoded() -> lurq::components::Row {
  let tile = |image: Image, caption: &str, file: &str| {
    Column::new()
      .width(160.0)
      .max_width(FILL)
      .spacing(6.0)
      .child(image.size(FILL, 132.0).cover().rounded(6.0).clip().border(line()))
      .child(text(caption, 12.0, FontWeight::SemiBold))
      .child(quiet(file, 11.0))
  };
  let placeholder = || Skeleton::block(Length::Fill, Length::Fill);
  samples()
    .child(tile(
      Image::from_bytes(ASSETS[0].bytes).placeholder_view(placeholder),
      "from_bytes",
      ASSETS[0].file,
    ))
    .child(tile(
      Image::from_source(ImageSource::Bytes(Arc::from(ASSETS[2].bytes)), ImageKey::new(GIF_KEY))
        .placeholder_view(placeholder),
      "from_source: its first frame",
      ASSETS[2].file,
    ))
    .child(tile(
      Image::from_bytes(BROKEN)
        .cache_key(ImageKey::new(0xB40C))
        .placeholder_view(placeholder)
        .error_view(|_| {
          Column::new()
            .size(FILL, FILL)
            .align_items(Alignment::Center)
            .justify(lurq::layout::layout_kind::Justify::Center)
            .background(PaletteColor::DestructiveMuted)
            .child(quiet("not a picture", 12.0).color(PaletteColor::Destructive))
        }),
      "error_view",
      "b\"not a picture\"",
    ))
}

pub struct ImagesPage {
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
  background: Signal<bool>,
  border: Signal<bool>,
}

impl ImagesPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (picture, fit_at, size, radius) = (self.picture.get(), self.fit.get(), self.size.get(), self.radius.get());
    let (background, border) = (self.background.get(), self.border.get());
    let picture = picture.min(ASSETS.len() - 1);
    let fit_name = FITS[fit_at.min(FITS.len() - 1)].0;
    let build = move || shown(picture, fit_at, size, radius, background, border);

    let asset = &ASSETS[picture];
    let mut code = vec![format!(
      "let pixels = ImageData::from_bytes(include_bytes!(\"{}\"))?;",
      asset.file
    )];
    if background {
      code.push("Stack::new()".to_owned());
      code.push(format!("    {}", size_code(size, true)));
      code.push("    .background_image(pixels)".to_owned());
      if fit_at != 0 {
        code.push(format!("    .background_size(BackgroundSize::{fit_name})"));
      }
    } else {
      code.push("Image::new(pixels)".to_owned());
      code.push(format!("    {}", size_code(size, false)));
      if fit_at != 0 {
        code.push(format!("    .fit(BackgroundSize::{fit_name})"));
      }
    }
    if radius > 0 {
      code.push(format!("    .rounded({radius}.0)"));
      code.push("    .clip()".to_owned());
    }
    if border {
      code.push("    .border(Border::inside(1.0, PaletteColor::Border))".to_owned());
    }
    if background {
      code.push(format!(
        "    .child(label(\"{}\").absolute_position(10.0, 10.0))",
        asset.label
      ));
    }

    let facts = match pixels(picture) {
      Some(pixels) => format!(
        "{} × {} px, {}",
        pixels.width(),
        pixels.height(),
        if pixels.is_animated() { "animated" } else { "still" }
      ),
      None => "decoding".to_owned(),
    };
    let tree = if background {
      vec![
        format!("stack   {}   background image", self.playground.measured()),
        "└─ label".to_owned(),
      ]
    } else {
      vec![format!("image   {}", self.playground.measured())]
    };

    let (set_picture, set_fit, set_size, set_radius) = (
      self.picture.clone(),
      self.fit.clone(),
      self.size.clone(),
      self.radius.clone(),
    );
    let reset = {
      let (picture, fit, size, radius, background, border) = (
        self.picture.clone(),
        self.fit.clone(),
        self.size.clone(),
        self.radius.clone(),
        self.background.clone(),
        self.border.clone(),
      );
      Arc::new(move || {
        picture.set(0);
        fit.set(2);
        size.set(2);
        radius.set(8);
        background.set(false);
        border.set(true);
      })
    };
    let controls = vec![
      prop(
        "picture",
        "ImageData",
        self
          .playground
          .select(0, &PICTURES, &picture, move |value| set_picture.set(value)),
      ),
      prop(
        if background { "background_size" } else { "fit" },
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
      flag("as a background", &self.background),
      flag("border", &self.border),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Image",
        make: Arc::new(build),
        shown: Shown::built(move |_| build()),
        fill: size == 3,
        forceable: false,
        facts,
        event: "",
        code,
        tree,
        controls,
        changed: [picture != 0, fit_at != 2, size != 2, radius != 8, background, !border]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

impl Component for ImagesPage {
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
      background: ctx.signal(false),
      border: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // Read so that the page is drawn again when the pictures are decoded.
    let _ = self.ready.get();
    let rows = [
      (
        "new",
        "ImageData",
        "",
        "An image of pixels the app has: decoded (ImageData::from_bytes, from_file), drawn, or made of RGBA \
         (ImageData::from_rgba). A GIF or a WebP decoded by ImageData::from_bytes keeps all its frames, and moves.",
      ),
      (
        "from_bytes / from_file",
        "impl Into<Arc<[u8]>> / PathBuf",
        "",
        "An image the node decodes itself, off the window's thread, and keeps in the process's cache of decoded \
         pictures. Of a GIF or a WebP, the first frame.",
      ),
      (
        "from_source",
        "ImageSource, ImageKey",
        "",
        "The same, with the key it is cached under given: the bytes are not hashed for one.",
      ),
      (
        "from_resource",
        "&str",
        "",
        "A file under the app's resource root (App::set_resource_root), or at a whole path, read and decoded by the \
         engine with every frame. A desktop build with http.",
      ),
      (
        "fit",
        "BackgroundSize",
        "Stretch",
        "How the pixels fill the box: stretched to it, contained in it whole, covering it and cut, or at their own \
         size. cover(), contain() and stretch() say the same.",
      ),
      (
        "placeholder_view",
        "Fn() -> impl Into<Element>",
        "",
        "What the node shows while it decodes, over its own background.",
      ),
      (
        "error_view",
        "Fn(&ImageDecodeError) -> impl Into<Element>",
        "",
        "What the node shows where the bytes are no picture it can decode.",
      ),
      (
        "downscale_to / cache_key",
        "u32, u32 / ImageKey",
        "",
        "Decodes a large picture no larger than a box of pixels, and says the key it is cached under.",
      ),
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
        "How a background fills its box. background_cover() and background_contain() say the same.",
      ),
    ];
    let playground = self.play(ctx);
    let room = self.frame.stage_room();
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Images"],
        title: "Images",
        description: "The engine's image node: pixels in a box, fitted to it as the node says, and the same pixels \
                      as the background of any box. It is deprecated for apps, which show a picture with \
                      ui::Picture: this is the node Picture draws with.",
        import: Some("use lurq::{components::Image, images::ImageData};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "One of the four pictures, as an image or as the background of a box. Choose how it fits a box of a \
           size: a width or a height alone gives the other side the picture's proportions.",
          playground,
        ),
        Section::new(
          "Plain images",
          "Each picture in a box of 160 by 132. The node stretches its pixels to its box unless told otherwise, so \
           a square picture in a wide box is wider than it was drawn.",
          self.frame.demo(
            0,
            plain(),
            "Image::new(pixels).size(160.0, 132.0)",
            &[
              "let pixels = ImageData::from_bytes(include_bytes!(\"skebob.jpg\"))?;",
              "Image::new(pixels)",
              "    .size(160.0, 132.0)",
              "    .rounded(6.0)",
              "    .clip()",
              "    .border(Border::inside(1.0, PaletteColor::Border))",
            ],
          ),
        ),
        Section::new(
          "Sized images",
          "A width alone: the height follows from the picture's proportions, as it would from a height alone.",
          self.frame.demo(
            1,
            sized(),
            "Image::new(pixels).width(120.0)",
            &[
              "Image::new(pixels)",
              "    .width(120.0)",
              "    .rounded(6.0)",
              "    .clip()",
            ],
          ),
        ),
        Section::new(
          "Intrinsic images",
          "No size at all: the image is as many points wide and high as the picture has pixels. The drawing with a \
           transparent ground is 510 by 818.",
          self.frame.demo(
            2,
            fit("engine-images-intrinsic", room, 540.0, intrinsic()),
            "Image::new(pixels)",
            &["Image::new(pixels).rounded(6.0).clip()"],
          ),
        ),
        Section::new(
          "Background images",
          "Any box can have pixels under its children. Cover fills the box and cuts what is past it; contain shows \
           the whole picture and the box's own fill beside it. The label is a child of the box.",
          self.frame.demo(
            3,
            backgrounds(),
            ".background_image(pixels).background_cover()",
            &[
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
          "Animated images",
          "A GIF and a WebP decoded with every frame: the node shows each frame for as long as the file says, and \
           the window draws while one is in view.",
          self.frame.demo(
            4,
            moving_images(),
            "Image::new(ImageData::from_bytes(gif)?)",
            &[
              "// Every frame, with how long each is shown.",
              "let pixels = ImageData::from_bytes(include_bytes!(\"six-seven.gif\"))?;",
              "assert!(pixels.is_animated());",
              "Image::new(pixels)",
              "    .size(Dimension::Pct(100.0), 210.0)",
              "    .cover()",
              "    .rounded(8.0)",
              "    .clip()",
            ],
          ),
        ),
        Section::new(
          "Animated backgrounds",
          "The same pictures under the children of a box: a background moves as an image does.",
          self.frame.demo(
            5,
            moving_backgrounds(),
            ".background_image(gif).background_cover()",
            &[
              "Stack::new()",
              "    .size(Dimension::Pct(100.0), 220.0)",
              "    .stack_align(StackAlignment::Center)",
              "    .background_image(ImageData::from_bytes(include_bytes!(\"six-seven.gif\"))?)",
              "    .background_cover()",
              "    .child(label(\"GIF, 48 frames\"))",
            ],
          ),
        ),
        Section::new(
          "From the app's resources",
          "A path among the app's resources (App::set_resource_root) is read by the engine off the window's thread \
           and decoded with every frame: a GIF read so moves, as an image and as a background. A path that is \
           whole is read as it is: the page writes its pictures to the system's folder for temporary files and \
           gives their paths.",
          self.frame.demo(
            7,
            resources(),
            "Image::from_resource(\"six-seven.gif\")",
            &[
              "app.set_resource_root(PathBuf::from(\"assets\"));",
              "",
              "Image::from_resource(\"six-seven.gif\")",
              "    .size(Dimension::Pct(100.0), 132.0)",
              "    .cover()",
              "",
              "Stack::new()",
              "    .size(Dimension::Pct(100.0), 132.0)",
              "    .background_image(\"six-seven.gif\")",
              "    .background_cover()",
            ],
          ),
        ),
        Section::new(
          "Decoded by the node",
          "Given bytes, the node decodes them itself, off the window's thread, and shows its placeholder until they \
           are. It keeps the first frame of a picture that moves, and shows its error view for bytes that are no \
           picture.",
          self.frame.demo(
            6,
            decoded(),
            "Image::from_bytes(bytes).placeholder_view(..)",
            &[
              "Image::from_bytes(include_bytes!(\"skebob.jpg\"))",
              "    .placeholder_view(|| Skeleton::block(Length::Fill, Length::Fill))",
              "",
              "// A key of the app's own: 700 KB are not hashed at each render.",
              "Image::from_source(ImageSource::Bytes(gif_bytes), ImageKey::new(GIF_KEY))",
              "",
              "Image::from_bytes(b\"not a picture\")",
              "    .error_view(|_| Column::new().background(PaletteColor::DestructiveMuted))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "How an image is made, how its pixels fill its box, and the background any box can have.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
