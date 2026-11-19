//! The page of the QR code: its playground, the levels side by side, sizes and a quiet zone, a logo over the
//! middle, and the card in the dark look.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{Icon, QrCode, QrLevel},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, field, prop},
};

const LEVELS: [(&str, QrLevel); 4] = [
  ("low", QrLevel::Low),
  ("medium", QrLevel::Medium),
  ("quartile", QrLevel::Quartile),
  ("high", QrLevel::High),
];

const LENGTHS: [(&str, f32); 3] = [("120", 120.0), ("160", 160.0), ("220", 220.0)];

const QUIET: [(&str, usize); 3] = [("0", 0), ("2", 2), ("4", 4)];

const ADDRESS: &str = "https://lurq.dev/pair?code=7F3K-92QD";

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .align_items(Alignment::Center)
    .child(shown)
    .child(quiet(caption, 12.0))
}

/// The logo of the examples: the mark of the app on its primary fill, in the light look's colors as a brand's
/// logo has them, since the code is light in either look.
fn logo() -> Row {
  let light = ThemePalette::light();
  Row::new()
    .size(24.0, 24.0)
    .rounded(RadiusSize::Sm)
    .background(light.get(PaletteColor::Primary))
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(
      Icon::new("bolt")
        .size(14.0)
        .color(light.get(PaletteColor::PrimaryForeground)),
    )
}

pub struct QrCodePage {
  frame: Frame,
  playground: Playground,
  text: Signal<String>,
  /// The index of the level in [`LEVELS`].
  level: Signal<usize>,
  /// The index of the side in [`LENGTHS`].
  length: Signal<usize>,
  /// The index of the quiet zone in [`QUIET`].
  quiet: Signal<usize>,
  media: Signal<bool>,
}

impl QrCodePage {
  /// The code the panel's props make, and the Rust that builds it.
  fn code(text: &str, level: usize, length: usize, quiet_zone: usize, media: bool) -> (QrCode, Vec<String>) {
    let mut code = QrCode::new(text);
    let mut call = vec![format!("QrCode::new({text:?})")];
    if level != 1 {
      code = code.level(LEVELS[level].1);
      call.push(format!("    .level(QrLevel::{:?})", LEVELS[level].1));
    }
    if length != 1 {
      code = code.length(LENGTHS[length].1);
      call.push(format!("    .length({:.1})", LENGTHS[length].1));
    }
    if quiet_zone != 0 {
      code = code.quiet_zone(QUIET[quiet_zone].1);
      call.push(format!("    .quiet_zone({})", QUIET[quiet_zone].1));
    }
    if media {
      code = code.media(logo());
      call.push("    .media(logo)".to_owned());
    }
    (code, call)
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let text = self.text.get();
    let (level, length, quiet_zone, media) = (self.level.get(), self.length.get(), self.quiet.get(), self.media.get());
    let made = {
      let text = text.clone();
      move || -> Element { Self::code(&text, level, length, quiet_zone, media).0.into() }
    };
    let (code, call) = Self::code(&text, level, length, quiet_zone, media);
    let matrix = lurq::ui::QrMatrix::encode(&text, if media { QrLevel::High } else { LEVELS[level].1 });
    let facts = match &matrix {
      Some(matrix) => format!(
        "version {}, {} modules, level {:?}",
        matrix.version(),
        matrix.size(),
        matrix.level()
      )
      .to_lowercase(),
      None => "too long for any code".to_owned(),
    };
    let (to_level, to_length, to_quiet) = (self.level.clone(), self.length.clone(), self.quiet.clone());
    let controls = vec![
      prop("text", "&str", field(&self.text)),
      prop(
        "level",
        "QrLevel",
        segmented_fill(
          &LEVELS
            .iter()
            .enumerate()
            .map(|(index, (name, _))| (*name, index))
            .collect::<Vec<_>>(),
          &level,
          move |chosen| to_level.set(chosen),
        ),
      ),
      prop(
        "length",
        "impl Into<Length>",
        segmented_fill(
          &LENGTHS
            .iter()
            .enumerate()
            .map(|(index, (name, _))| (*name, index))
            .collect::<Vec<_>>(),
          &length,
          move |chosen| to_length.set(chosen),
        ),
      ),
      prop(
        "quiet_zone",
        "usize",
        segmented_fill(
          &QUIET
            .iter()
            .enumerate()
            .map(|(index, (name, _))| (*name, index))
            .collect::<Vec<_>>(),
          &quiet_zone,
          move |chosen| to_quiet.set(chosen),
        ),
      ),
      crate::playground::flag("media", &self.media),
    ];
    let tree = vec![
      format!(
        "root      {}   img \"QR code\", described by the text",
        self.playground.measured()
      ),
      "├─ module × n   foreground of the light palette, on whole device pixels".to_owned(),
      format!(
        "└─ media{}",
        if media {
          "   a well of the light fill"
        } else {
          "   (none)"
        }
      ),
    ];
    let reset = {
      let (text, level, length, quiet, media) = (
        self.text.clone(),
        self.level.clone(),
        self.length.clone(),
        self.quiet.clone(),
        self.media.clone(),
      );
      Arc::new(move || {
        text.set(ADDRESS.to_owned());
        level.set(1);
        length.set(1);
        quiet.set(0);
        media.set(false);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "QR Code",
        make: Arc::new(made),
        shown: Shown::of(self.playground.styled(code)),
        fill: false,
        forceable: false,
        facts,
        event: "",
        code: call,
        tree,
        controls,
        changed: [text != ADDRESS, level != 1, length != 1, quiet_zone != 0, media]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

impl Component for QrCodePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      text: ctx.signal(ADDRESS.to_owned()),
      level: ctx.signal(1),
      length: ctx.signal(1),
      quiet: ctx.signal(0),
      media: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let levels = Row::new()
      .width(FILL)
      .spacing(24.0)
      .wrap()
      .align_items(Alignment::End)
      .with_children(LEVELS.map(|(name, level)| item(QrCode::new(ADDRESS).level(level).length(140.0), name)));
    let sizes = Row::new()
      .width(FILL)
      .spacing(24.0)
      .wrap()
      .align_items(Alignment::End)
      .child(item(QrCode::new("lurq").length(97.0), "97, a short text"))
      .child(item(QrCode::new(ADDRESS), "160, the default"))
      .child(item(
        QrCode::new(ADDRESS).length(220.0).quiet_zone(4),
        "220, a quiet zone of 4",
      ));
    let media = Row::new()
      .width(FILL)
      .spacing(24.0)
      .wrap()
      .align_items(Alignment::End)
      .child(item(QrCode::new(ADDRESS).media(logo()), "a logo, at the highest level"))
      .child(item(
        QrCode::new("WIFI:T:WPA;S:lurq-guest;P:correct horse battery staple;;")
          .label("Join the guest network")
          .media(logo())
          .length(200.0),
        "a network to join",
      ));
    let rows = [
      (
        "new(text)",
        "QrCode",
        "",
        "The code of the text's UTF-8 bytes, in the smallest version that holds them.",
      ),
      (
        "level",
        "QrLevel",
        "Medium",
        "How much can be lost and the code still read: Low, Medium, Quartile, High. A higher one is taken where \
         it fits in the same size; a text too long for the one asked for is drawn at the highest that holds it.",
      ),
      (
        "length",
        "impl Into<Length>",
        "160",
        "The side of the card. Modules are whole logical pixels, drawn on whole device pixels; the code stands \
         in the middle of what is left.",
      ),
      (
        "quiet_zone",
        "usize",
        "0",
        "Modules of light around the code inside the card's padding of 10, which is light already.",
      ),
      (
        "media",
        "impl Into<Element>",
        "",
        "A logo over the middle, in a well of the light fill; the code is made at High.",
      ),
      (
        "label",
        "&str",
        "\"QR code\"",
        "What tooling calls the image; its description is the text.",
      ),
      (
        "id, key, style, rules, state",
        "",
        "",
        "As every themed builder has them.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "QR Code"],
        title: "QR Code",
        description: "A text an app gives, drawn as a code a phone's camera reads: dark modules on the light fill \
                      of a card, in the light look and the dark one alike.",
        import: Some("use lurq::ui::{QrCode, QrLevel};"),
        status: Some("Stable"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The text, the level, the side, a quiet zone and a logo. The facts say the version the text takes.",
          playground,
        ),
        Section::new(
          "Levels",
          "One address at each level, at one side: the more of it can be lost, the more modules and the smaller \
           each.",
          self.frame.demo(
            0,
            levels,
            "QrCode::new(address).level(QrLevel::Quartile)",
            &[
              "QrCode::new(address).level(QrLevel::Low).length(140.0)",
              "QrCode::new(address).length(140.0)",
              "QrCode::new(address).level(QrLevel::Quartile).length(140.0)",
              "QrCode::new(address).level(QrLevel::High).length(140.0)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "The board's 97, the default of 160, and 220 with a quiet zone of four modules for a code that is \
           printed.",
          self.frame.demo(
            1,
            sizes,
            "QrCode::new(address).length(220.0).quiet_zone(4)",
            &[
              "QrCode::new(\"lurq\").length(97.0)",
              "QrCode::new(address)",
              "QrCode::new(address).length(220.0).quiet_zone(4)",
            ],
          ),
        ),
        Section::new(
          "Logo",
          "A mark over the middle, in a well of the card's fill. The code is made at the highest level, which \
           still reads with it there.",
          self.frame.demo(
            2,
            media,
            "QrCode::new(address).media(logo)",
            &[
              "QrCode::new(address).media(logo)",
              "QrCode::new(\"WIFI:T:WPA;S:lurq-guest;P:correct horse battery staple;;\")",
              "    .label(\"Join the guest network\")",
              "    .media(logo)",
              "    .length(200.0)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a QR code is made with and given.",
          props_table(&rows, &["new(text)"], self.frame.room()),
        ),
      ],
    )
  }
}
