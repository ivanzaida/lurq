//! The page of the avatar: its playground, the board's sizes and shapes, a
//! picture and one that fails, the dot of presence, groups, and a person's
//! name beside an avatar.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Image, Row},
  core::Signal,
  images::{ImageData, ImageKey},
  layout::{Alignment, text_style::FontWeight},
  node::{Element, border::Border},
  ui::{Avatar, AvatarGroup, AvatarGroupPart, AvatarPresence, AvatarShape, AvatarSize, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, quiet, text},
  playground::{Playground, Spec, divider, field, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(10.0)
    .child(Row::new().padding(2.0).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

/// The people of the page's groups.
const PEOPLE: [&str; 6] = [
  "Ivan Zaida",
  "Anna K.",
  "Mark R.",
  "Ivy Chen",
  "Ivo Kovac",
  "Lena Quinn",
];

/// What the playground's avatar shows in place of its initials.
#[derive(Clone, Copy, PartialEq)]
enum Picture {
  None,
  /// A portrait the page draws.
  Portrait,
  /// Bytes that are no picture: the decode fails and the initials stay.
  Broken,
}

const PICTURES: [(&str, Picture); 3] = [
  ("None", Picture::None),
  ("Portrait", Picture::Portrait),
  ("Broken", Picture::Broken),
];

const SIZES: [(&str, AvatarSize); 5] = [
  ("Xs · 20", AvatarSize::Xs),
  ("Sm · 24", AvatarSize::Sm),
  ("Md · 32", AvatarSize::Md),
  ("Lg · 40", AvatarSize::Lg),
  ("Xl · 56", AvatarSize::Xl),
];

const SHAPES: [(&str, AvatarShape); 2] = [("Circle", AvatarShape::Circle), ("Square", AvatarShape::Square)];

const PRESENCES: [(&str, Option<AvatarPresence>); 5] = [
  ("None", None),
  ("Online", Some(AvatarPresence::Online)),
  ("Away", Some(AvatarPresence::Away)),
  ("Busy", Some(AvatarPresence::Busy)),
  ("Offline", Some(AvatarPresence::Offline)),
];

const MAXES: [(&str, usize); 5] = [("None", 0), ("2", 2), ("3", 3), ("4", 4), ("5", 5)];

/// A portrait of nobody, 64 pixels square: a head and shoulders, light, over a
/// gradient from `top` to `bottom`. The gallery has no pictures of people.
fn portrait(top: [f32; 3], bottom: [f32; 3]) -> ImageData {
  const SIDE: u32 = 64;
  let mut rgba = Vec::with_capacity((SIDE * SIDE * 4) as usize);
  for y in 0..SIDE {
    for x in 0..SIDE {
      let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
      let along = py / SIDE as f32;
      // How far inside the head or the shoulders the pixel is, in pixels: more than half a pixel is all of it.
      let head = 11.0 - ((px - 32.0).powi(2) + (py - 26.0).powi(2)).sqrt();
      let shoulders = (1.0 - (((px - 32.0) / 22.0).powi(2) + ((py - 64.0) / 22.0).powi(2)).sqrt()) * 22.0;
      let cover = (head.max(shoulders) + 0.5).clamp(0.0, 1.0) * 0.82;
      for channel in 0..3 {
        let ground = top[channel] + (bottom[channel] - top[channel]) * along;
        rgba.push(((ground + (1.0 - ground) * cover) * 255.0).round() as u8);
      }
      rgba.push(255);
    }
  }
  ImageData::from_rgba(rgba, SIDE, SIDE)
}

/// Bytes that are no picture: what a broken file is to the decoder.
const BROKEN: &[u8] = b"not a picture";

/// The broken picture, under a key of its own: the decoder's cache keeps that it failed.
fn broken() -> Image {
  Image::from_bytes(BROKEN).cache_key(ImageKey::new(0x6A11_E4E7))
}

pub struct AvatarPage {
  frame: Frame,
  playground: Playground,
  name: Signal<String>,
  picture: Signal<Picture>,
  size: Signal<AvatarSize>,
  shape: Signal<AvatarShape>,
  presence: Signal<Option<AvatarPresence>>,
  max: Signal<usize>,
  /// The portraits the page draws, once: the same pixels each render.
  portraits: [ImageData; 3],
}

impl AvatarPage {
  /// The playground: the avatar as the panel's props make it, and a group of six with the most it shows.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let name = self.name.get();
    let (picture, size, shape) = (self.picture.get(), self.size.get(), self.shape.get());
    let (presence, max) = (self.presence.get(), self.max.get());
    let portrait = self.portraits[0].clone();
    let avatar = {
      let (name, portrait) = (name.clone(), portrait.clone());
      move || {
        let avatar = Avatar::new(&name).size(size).shape(shape).presence(presence);
        match picture {
          Picture::None => avatar,
          Picture::Portrait => avatar.image(Image::new(portrait.clone())),
          Picture::Broken => avatar.image(broken()),
        }
      }
    };
    let group = move || {
      let group = AvatarGroup::new(PEOPLE.map(Avatar::new)).size(size).label("Reviewers");
      if max > 0 { group.max(max) } else { group }
    };
    let shown = {
      let (avatar, group) = (avatar.clone(), group.clone());
      move || -> Element {
        Row::new()
          .spacing(40.0)
          .padding(4.0)
          .align_items(Alignment::Center)
          .overflow_visible()
          .child(avatar())
          .child(group())
          .into()
      }
    };

    let size_name = SIZES
      .iter()
      .find(|(_, held)| *held == size)
      .map_or("Md · 32", |(label, _)| *label);
    let mut code = vec![format!("Avatar::new({name:?})")];
    match picture {
      Picture::None => {}
      Picture::Portrait => code.push("    .image(Image::from_file(\"ivan.png\"))".to_owned()),
      Picture::Broken => code.push("    .image(Image::from_bytes(broken_bytes))".to_owned()),
    }
    if size != AvatarSize::Md {
      code.push(format!("    .size(AvatarSize::{size:?})"));
    }
    if shape != AvatarShape::Circle {
      code.push(format!("    .shape(AvatarShape::{shape:?})"));
    }
    if let Some(presence) = presence {
      code.push(format!("    .presence(AvatarPresence::{presence:?})"));
    }
    code.push(String::new());
    code.push("AvatarGroup::new(people.iter().map(|name| Avatar::new(name)))".to_owned());
    if size != AvatarSize::Md {
      code.push(format!("    .size(AvatarSize::{size:?})"));
    }
    if max > 0 {
      code.push(format!("    .max({max})"));
    }
    code.push("    .label(\"Reviewers\")".to_owned());

    let side = match size {
      AvatarSize::Xs => 20,
      AvatarSize::Sm => 24,
      AvatarSize::Md => 32,
      AvatarSize::Lg => 40,
      AvatarSize::Xl => 56,
    };
    let what = match picture {
      Picture::None if name.trim().is_empty() => "icon of a person".to_owned(),
      Picture::None => "initials".to_owned(),
      Picture::Portrait => "image, clipped to the face".to_owned(),
      Picture::Broken => "image that failed: its error view, the initials".to_owned(),
    };
    let mut tree = vec![
      format!("avatar (img {name:?})   {}", self.playground.measured()),
      format!(
        "{} face       {side} × {side} px · {}",
        if presence.is_some() { "├─" } else { "└─" },
        if shape == AvatarShape::Square {
          "radius-md"
        } else {
          "round"
        }
      ),
      format!("{}  └─ {what}", if presence.is_some() { "│" } else { " " }),
    ];
    if let Some(presence) = presence {
      tree.push(format!("└─ presence   {}", presence.as_str()));
    }
    let hidden = if max > 0 { PEOPLE.len().saturating_sub(max) } else { 0 };
    tree.push(format!(
      "group (\"Reviewers\")   {} avatars{}",
      PEOPLE.len() - hidden,
      if hidden > 0 {
        format!(" and +{hidden}")
      } else {
        String::new()
      }
    ));

    let mut facts = vec![size_name.to_owned()];
    if let Some(presence) = presence {
      facts.push(presence.as_str().to_owned());
    }
    let (to_name, to_picture, to_size) = (self.name.clone(), self.picture.clone(), self.size.clone());
    let (to_shape, to_presence, to_max) = (self.shape.clone(), self.presence.clone(), self.max.clone());
    let (pick_picture, pick_size, pick_shape) = (self.picture.clone(), self.size.clone(), self.shape.clone());
    let (pick_presence, pick_max) = (self.presence.clone(), self.max.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Avatar",
        make: Arc::new(shown.clone()),
        shown: shown(),
        fill: false,
        forceable: false,
        facts: facts.join(" · "),
        event: "",
        code,
        tree,
        controls: vec![
          prop("name", "&str", field(&self.name)),
          prop(
            "image",
            "Image",
            self
              .playground
              .select(0, &PICTURES, &picture, move |which| pick_picture.set(which)),
          ),
          divider(),
          prop(
            "size",
            "AvatarSize",
            self
              .playground
              .select(1, &SIZES, &size, move |which| pick_size.set(which)),
          ),
          prop(
            "shape",
            "AvatarShape",
            self
              .playground
              .select(2, &SHAPES, &shape, move |which| pick_shape.set(which)),
          ),
          prop(
            "presence",
            "AvatarPresence",
            self
              .playground
              .select(3, &PRESENCES, &presence, move |which| pick_presence.set(which)),
          ),
          divider(),
          prop(
            "group max",
            "usize",
            self
              .playground
              .select(4, &MAXES, &max, move |which| pick_max.set(which)),
          ),
        ],
        changed: [
          name != "Ivan Zaida",
          picture != Picture::None,
          size != AvatarSize::Md,
          shape != AvatarShape::Circle,
          presence.is_some(),
          max != 3,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_name.set("Ivan Zaida".to_owned());
          to_picture.set(Picture::None);
          to_size.set(AvatarSize::Md);
          to_shape.set(AvatarShape::Circle);
          to_presence.set(None);
          to_max.set(3);
        }),
      },
    )
  }

  /// The board's persona: an avatar with its presence, and a name over a line under it.
  fn persona(name: &str, line_under: &str, presence: AvatarPresence) -> Row {
    Row::new()
      .spacing(10.0)
      .align_items(Alignment::Center)
      .overflow_visible()
      .child(Avatar::new(name).presence(presence))
      .child(
        Column::new()
          .spacing(1.0)
          .child(text(name, 14.0, FontWeight::Medium))
          .child(text(line_under, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground)),
      )
  }
}

impl Component for AvatarPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 5),
      name: ctx.signal("Ivan Zaida".to_owned()),
      picture: ctx.signal(Picture::None),
      size: ctx.signal(AvatarSize::Md),
      shape: ctx.signal(AvatarShape::Circle),
      presence: ctx.signal(None),
      max: ctx.signal(3),
      portraits: [
        portrait([0.42, 0.55, 0.93], [0.16, 0.25, 0.62]),
        portrait([0.96, 0.62, 0.38], [0.74, 0.27, 0.32]),
        portrait([0.40, 0.80, 0.66], [0.10, 0.45, 0.45]),
      ],
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let sizes = SIZES
      .iter()
      .fold(items(), |row, (label, size)| {
        row.child(item(Avatar::new("Ivan Zaida").size(*size), label))
      })
      .child(item(Avatar::new("Lurq Team").shape(AvatarShape::Square), "square"))
      .child(item(Avatar::new("Build bot").icon(icons::TERMINAL), "icon"))
      .child(item(Avatar::new(""), "no name: a person"));
    let [blue, orange, green] = self.portraits.clone();
    let pictures = items()
      .child(item(Avatar::new("Ivan Zaida").image(Image::new(blue.clone())), "round"))
      .child(item(
        Avatar::new("Anna K.")
          .image(Image::new(orange.clone()))
          .size(AvatarSize::Xl),
        "56",
      ))
      .child(item(
        Avatar::new("Lurq Team")
          .image(Image::new(green.clone()))
          .shape(AvatarShape::Square)
          .size(AvatarSize::Lg),
        "square",
      ))
      .child(item(Avatar::new("Mark R.").image(broken()), "fails: the initials stay"));
    let presences = PRESENCES[1..].iter().fold(items(), |row, (label, presence)| {
      row.child(item(
        Avatar::new("Ivan Zaida")
          .image(Image::new(blue.clone()))
          .presence(*presence),
        &label.to_lowercase(),
      ))
    });
    let groups = Column::new()
      .spacing(24.0)
      .child(item(
        AvatarGroup::new(PEOPLE.map(Avatar::new)).max(3).label("Reviewers"),
        "max 3 of 6",
      ))
      .child(item(
        AvatarGroup::new([
          Avatar::new("Ivan Zaida").image(Image::new(blue.clone())),
          Avatar::new("Anna K.").image(Image::new(orange.clone())),
          Avatar::new("Ivy Chen").image(Image::new(green.clone())),
          Avatar::new("Mark R."),
        ])
        .size(AvatarSize::Lg)
        .label("Editors"),
        "40, pictures and initials",
      ))
      .child(item(
        AvatarGroup::new(PEOPLE.map(Avatar::new))
          .size(AvatarSize::Sm)
          .max(4)
          .label("Watching"),
        "24, max 4",
      ));
    let on_card = Row::new()
      .padding(16.0)
      .rounded(RadiusSize::Lg)
      .background(PaletteColor::Card)
      .border(line())
      .overflow_visible()
      .child(
        AvatarGroup::new(PEOPLE.map(Avatar::new))
          .max(3)
          .label("Reviewers")
          .style(AvatarGroupPart::Item, |s| {
            s.border(Border::outside(2.0, PaletteColor::Card))
          }),
      );
    let people = items()
      .child(item(
        Self::persona("Ivan Zaida", "Available · Kyiv", AvatarPresence::Online),
        "persona",
      ))
      .child(item(
        Self::persona("Ivy Chen", "Away · back at 15:00", AvatarPresence::Away),
        "persona · away",
      ));
    let rows = [
      (
        "name",
        "&str",
        "",
        "Who it is: its initials are the first letters of the first word and the last, and it is what tooling \
         calls the avatar. A name with no letters shows the icon of a person.",
      ),
      (
        "image",
        "Image",
        "",
        "A picture in place of the initials, filling the face and clipped to its shape. While it decodes and \
         when it fails, the initials or the icon show.",
      ),
      (
        "icon",
        "&str",
        "",
        "An icon in place of the initials, by its name in the theme: for what is not a person.",
      ),
      ("size", "AvatarSize", "Md", "Xs 20, Sm 24, Md 32, Lg 40, Xl 56."),
      (
        "shape",
        "AvatarShape",
        "Circle",
        "Circle, or Square with the corners of a control.",
      ),
      (
        "presence",
        "Option<AvatarPresence>",
        "None",
        "The dot at the bottom end: online, away, busy or offline.",
      ),
      (
        "AvatarGroup::max",
        "usize",
        "",
        "The most avatars a group shows; after them it shows how many it leaves out, \"+3\".",
      ),
      (
        "AvatarGroup::label",
        "&str",
        "",
        "What the group is called to tooling: without one, the names it shows.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Avatar"],
        title: "Avatar",
        description: "Who a person is, at a glance: the initials of the name on a fill, a picture in their place, \
                      whether the person is there; and a group of them laid over one another.",
        import: Some("use lurq::ui::{Avatar, AvatarGroup};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Avatar is a control on the right, and the most a group of six shows. A broken picture \
           shows what an avatar does when its picture fails: the initials stay.",
          playground,
        ),
        Section::new(
          "Sizes and shapes",
          "Five sizes, round or square. An icon takes the place of the initials for what is not a person, and a \
           name with no letters shows a person.",
          self.frame.demo(
            0,
            sizes,
            ".size(AvatarSize::Xl)",
            &[
              "Avatar::new(\"Ivan Zaida\").size(AvatarSize::Xs)",
              "Avatar::new(\"Ivan Zaida\").size(AvatarSize::Sm)",
              "Avatar::new(\"Ivan Zaida\")",
              "Avatar::new(\"Ivan Zaida\").size(AvatarSize::Lg)",
              "Avatar::new(\"Ivan Zaida\").size(AvatarSize::Xl)",
              "Avatar::new(\"Lurq Team\").shape(AvatarShape::Square)",
              "Avatar::new(\"Build bot\").icon(icons::TERMINAL)",
              "Avatar::new(\"\")",
            ],
          ),
        ),
        Section::new(
          "Pictures",
          "A picture fills the face and is clipped to its shape. The app builds the image as it likes, from \
           pixels, a file or bytes; while it decodes and when it fails, the initials show in its place.",
          self.frame.demo(
            1,
            pictures,
            ".image(Image::from_file(path))",
            &[
              "Avatar::new(\"Ivan Zaida\").image(Image::from_file(\"ivan.png\"))",
              "Avatar::new(\"Anna K.\")",
              "    .image(Image::from_file(\"anna.png\").downscale_to(112, 112))",
              "    .size(AvatarSize::Xl)",
              "Avatar::new(\"Lurq Team\")",
              "    .image(Image::new(pixels))",
              "    .shape(AvatarShape::Square)",
              "    .size(AvatarSize::Lg)",
              "// Bytes that are no picture: the decode fails, and the initials stay.",
              "Avatar::new(\"Mark R.\").image(Image::from_bytes(broken_bytes))",
            ],
          ),
        ),
        Section::new(
          "Presence",
          "A dot at the bottom end of the face says whether the person is there, ringed in the background's color. \
           Tooling hears it after the name: \"Ivan Zaida, away\".",
          self.frame.demo(
            2,
            presences,
            ".presence(AvatarPresence::Online)",
            &[
              "Avatar::new(\"Ivan Zaida\").image(portrait).presence(AvatarPresence::Online)",
              "Avatar::new(\"Ivan Zaida\").image(portrait).presence(AvatarPresence::Away)",
              "Avatar::new(\"Ivan Zaida\").image(portrait).presence(AvatarPresence::Busy)",
              "Avatar::new(\"Ivan Zaida\").image(portrait).presence(AvatarPresence::Offline)",
            ],
          ),
        ),
        Section::new(
          "Groups",
          "Each avatar lies over the end of the one before it, ringed in the background's color. With a most, the \
           group shows that many and how many it leaves out as the last. The group gives its avatars its size.",
          self.frame.demo(
            3,
            groups,
            "AvatarGroup::new(avatars).max(3)",
            &[
              "let people = [\"Ivan Zaida\", \"Anna K.\", \"Mark R.\", \"Ivy Chen\", \"Ivo Kovac\", \"Lena Quinn\"];",
              "AvatarGroup::new(people.map(Avatar::new)).max(3).label(\"Reviewers\")",
              "",
              "AvatarGroup::new([",
              "    Avatar::new(\"Ivan Zaida\").image(ivan),",
              "    Avatar::new(\"Anna K.\").image(anna),",
              "    Avatar::new(\"Ivy Chen\").image(ivy),",
              "    Avatar::new(\"Mark R.\"),",
              "])",
              ".size(AvatarSize::Lg)",
              ".label(\"Editors\")",
              "",
              "AvatarGroup::new(people.map(Avatar::new)).size(AvatarSize::Sm).max(4).label(\"Watching\")",
            ],
          ),
        ),
        Section::new(
          "On a card",
          "The ring is the color of the surface behind the group: on a card, the card's.",
          self.frame.demo(
            4,
            on_card,
            ".style(AvatarGroupPart::Item, ..)",
            &[
              "AvatarGroup::new(people.map(Avatar::new))",
              "    .max(3)",
              "    .label(\"Reviewers\")",
              "    .style(AvatarGroupPart::Item, |s| s.border(Border::outside(2.0, PaletteColor::Card)))",
            ],
          ),
        ),
        Section::new(
          "Beside a name",
          "A person in a list or a menu: the avatar with its presence, the name and a line under it. It is an \
           avatar beside two texts, built where it is used.",
          self.frame.demo(
            5,
            people,
            "Row::new().child(Avatar::new(name))",
            &[
              "Row::new()",
              "    .spacing(10.0)",
              "    .align_items(Alignment::Center)",
              "    .child(Avatar::new(\"Ivan Zaida\").presence(AvatarPresence::Online))",
              "    .child(",
              "        Column::new()",
              "            .spacing(1.0)",
              "            .child(Text::new(\"Ivan Zaida\"))",
              "            .child(Text::new(\"Available · Kyiv\").color(PaletteColor::MutedForeground)),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an avatar and a group are given.",
          props_table(&rows, &["name"], self.frame.room()),
        ),
      ],
    )
  }
}
