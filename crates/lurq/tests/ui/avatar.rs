//! An avatar: the initials of the name on a face of each size and shape, the
//! icon in their place, the dot of presence, a picture clipped to the face and
//! the initials while it fails; a group laid over one another with rings and
//! the count of who it leaves out; and a theme's rules and anatomy.

use std::sync::Arc;

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  layout::quad::{Quad, QuadContent},
  node::{border::BorderPlacement, color::Color},
  ui::{Avatar, AvatarGroup, AvatarPart, AvatarPresence, AvatarRenderer, AvatarShape, AvatarSize, AvatarView, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The character the theme's icon font draws `name` with.
fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find(|(held, _)| *held == name)
    .map(|(_, glyph)| glyph.to_string())
    .unwrap_or_else(|| panic!("no icon {name}"))
}

/// The radius of the top start corner of a quad.
fn corner(quad: &Quad) -> f32 {
  quad.border_radius.map_or(0.0, |radius| radius.top_left)
}

/// The filled square of `side` whose bottom end corner is at the bottom end
/// corner of the avatar with `id`: its dot of presence.
fn dot(stage: &mut Stage, id: &str, side: f32) -> Option<Quad> {
  let (x, y, width, height) = stage.bounds(id);
  stage.quads().into_iter().find(|quad| {
    matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
      && (quad.width, quad.height) == (side, side)
      && (quad.x + quad.width, quad.y + quad.height) == (x + width, y + height)
  })
}

fn color_of(quad: &Quad) -> Color {
  match quad.content {
    QuadContent::Rect { color, .. } => color,
    _ => panic!("a filled rectangle"),
  }
}

#[test]
fn an_avatar_is_the_initials_of_the_name_on_a_round_muted_face() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Avatar::new("Ivan Zaida").id("ivan"))
      .into()
  });
  let (x, y, width, height) = stage.bounds("ivan");
  assert_eq!((width, height), (32.0, 32.0));
  let face = stage.fill("ivan").expect("the face is filled");
  assert_eq!(color_of(&face), light(PaletteColor::Muted));
  assert_eq!(corner(&face), 999.0, "round");

  // The letters are 12, medium, in the foreground, in the middle of the face.
  assert_eq!(stage.text("IZ"), (light(PaletteColor::Foreground), 12.0));
  let letters = stage.text_quad("IZ").unwrap();
  let middle = (letters.x + letters.width / 2.0, letters.y + letters.height / 2.0);
  assert!((middle.0 - (x + 16.0)).abs() < 0.5, "centered across: {middle:?}");
  assert!((middle.1 - (y + 16.0)).abs() < 0.5, "centered down: {middle:?}");
  assert!(dot(&mut stage, "ivan", 10.0).is_none(), "no dot without a presence");
}

#[test]
fn each_size_has_the_side_and_the_letters_of_the_board() {
  let sizes = [
    (AvatarSize::Xs, "xs", "Anna", 20.0, 9.0),
    (AvatarSize::Sm, "sm", "Bo", 24.0, 10.0),
    (AvatarSize::Md, "md", "Cy", 32.0, 12.0),
    (AvatarSize::Lg, "lg", "Di", 40.0, 14.0),
    (AvatarSize::Xl, "xl", "Ed", 56.0, 18.0),
  ];
  let mut stage = Stage::new(move |_| {
    sizes
      .iter()
      // A name of its own for each, so that each one's letters are found.
      .fold(Row::new().padding(8.0).spacing(12.0), |row, (size, id, first, _, _)| {
        row.child(Avatar::new(&format!("{first} Zaida")).size(*size).id(*id))
      })
      .into()
  });
  for (_, id, first, side, letters) in sizes {
    let (.., width, height) = stage.bounds(id);
    assert_eq!((width, height), (side, side), "{id}");
    assert_eq!(corner(&stage.fill(id).unwrap()), 999.0, "{id}");
    let initials = format!("{}Z", &first[..1]);
    assert_eq!(stage.text(&initials).1, letters, "{id}");
  }
}

#[test]
fn a_square_avatar_has_the_corners_of_a_control() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Avatar::new("Lurq Team").shape(AvatarShape::Square).id("team"))
      .into()
  });
  let face = stage.fill("team").unwrap();
  assert_eq!(corner(&face), 6.0, "radius-md");
  assert_eq!(color_of(&face), light(PaletteColor::Muted));
  assert!(stage.paints("LT"));
}

#[test]
fn an_icon_takes_the_place_of_the_letters() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Avatar::new("").id("nobody"))
      .child(Avatar::new("Build bot").icon(icons::TERMINAL).id("bot"))
      .child(
        Avatar::new("Design team")
          .icon(icons::USERS)
          .size(AvatarSize::Lg)
          .id("team"),
      )
      .into()
  });
  // Half the side, in the muted foreground, on the muted face.
  assert_eq!(
    stage.text(&glyph(icons::USER)),
    (light(PaletteColor::MutedForeground), 16.0),
    "a name with no letters shows a person"
  );
  assert_eq!(stage.text(&glyph(icons::TERMINAL)).1, 16.0);
  assert!(!stage.paints("BB"), "the icon is in the place of the letters");
  assert_eq!(stage.text(&glyph(icons::USERS)).1, 20.0, "half of 40");
  assert_eq!(stage.fill_color("bot"), Some(light(PaletteColor::Muted)));
}

#[test]
fn the_dot_of_presence_stands_at_the_bottom_end_ringed_in_the_background() {
  let presences = [
    (AvatarPresence::Online, "online", PaletteColor::Success),
    (AvatarPresence::Away, "away", PaletteColor::Warning),
    (AvatarPresence::Busy, "busy", PaletteColor::Destructive),
    (AvatarPresence::Offline, "offline", PaletteColor::MutedForeground),
  ];
  let shown = presences.clone();
  let mut stage = Stage::new(move |_| {
    shown
      .iter()
      .fold(Row::new().padding(8.0).spacing(12.0), |row, (presence, id, _)| {
        row.child(Avatar::new("Ivan Zaida").presence(*presence).id(*id))
      })
      .child(
        Avatar::new("Ivy Chen")
          .size(AvatarSize::Xl)
          .presence(AvatarPresence::Online)
          .id("large"),
      )
      .into()
  });
  for (_, id, role) in presences {
    let (x, y, ..) = stage.bounds(id);
    let dot = dot(&mut stage, id, 10.0).unwrap_or_else(|| panic!("{id} has a dot of 10"));
    assert_eq!(
      (dot.x - x, dot.y - y),
      (22.0, 22.0),
      "{id}: at the bottom end of the face"
    );
    assert_eq!(color_of(&dot), light(role), "{id}");
    assert_eq!(corner(&dot), 999.0, "{id}: round");
    let ring = dot.border.and_then(|border| border.top).expect("a ring");
    assert_eq!(ring.width, 2.0, "{id}");
    assert_eq!(ring.color, light(PaletteColor::Background), "{id}");
    assert_eq!(ring.placement, BorderPlacement::Outside, "{id}: around the dot");
  }
  // The avatar is as large as its face: the dot stands over it and adds nothing.
  let (.., width, height) = stage.bounds("online");
  assert_eq!((width, height), (32.0, 32.0));
  assert!(
    dot(&mut stage, "large", 14.0).is_some(),
    "a larger dot on a larger face"
  );
}

#[test]
fn a_group_lays_its_avatars_over_one_another_each_ringed_in_the_background() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(
        AvatarGroup::new(
          ["Ivan Zaida", "Anna K.", "Mark R."]
            .iter()
            .enumerate()
            .map(|(index, name)| Avatar::new(name).id(format!("a{index}"))),
        )
        .id("group"),
      )
      .into()
  });
  let (x, _, width, height) = stage.bounds("group");
  assert_eq!((width, height), (80.0, 32.0), "three of 32, each 8 over the one before");
  for index in 0..3 {
    let id = format!("a{index}");
    let (start, ..) = stage.bounds(&id);
    assert_eq!(start - x, 24.0 * index as f32, "{id}");
    let ring = stage
      .border(&id)
      .and_then(|quad| quad.border)
      .and_then(|border| border.top)
      .unwrap_or_else(|| panic!("{id} is ringed"));
    assert_eq!(ring.width, 2.0);
    assert_eq!(ring.color, light(PaletteColor::Background));
    assert_eq!(ring.placement, BorderPlacement::Outside, "outside the face");
  }
  // A later avatar is painted over the one before it.
  let quads = stage.quads();
  let at = |content: &str| {
    quads
      .iter()
      .position(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content))
      .unwrap()
  };
  assert!(at("IZ") < at("AK") && at("AK") < at("MR"));
}

#[test]
fn a_group_with_a_most_shows_how_many_it_leaves_out_as_the_last() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(
        AvatarGroup::new(
          [
            "Ivan Zaida",
            "Anna K.",
            "Mark R.",
            "Ivy Chen",
            "Ivo Kovac",
            "Lena Quinn",
          ]
          .map(Avatar::new),
        )
        .max(3)
        .label("Reviewers")
        .id("group"),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("group");
  assert_eq!((width, height), (104.0, 32.0), "the board's group: three and the count");
  for shown in ["IZ", "AK", "MR", "+3"] {
    assert!(stage.paints(shown), "{shown}");
  }
  for left_out in ["IC", "IK", "LQ"] {
    assert!(!stage.paints(left_out), "{left_out}");
  }
  // The count is the last, in the secondary fill, ringed as the others are.
  let count = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && (quad.x - x, quad.y - y, quad.width, quad.height) == (72.0, 0.0, 32.0, 32.0)
    })
    .expect("the count's face");
  assert_eq!(color_of(&count), light(PaletteColor::Secondary));
  assert_eq!(stage.text("+3"), (light(PaletteColor::Foreground), 12.0));
}

#[test]
fn a_group_gives_its_avatars_its_size_and_none_beyond_its_most() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(
        AvatarGroup::new(["Ivan Zaida", "Anna K."].map(|name| Avatar::new(name).size(AvatarSize::Xl)))
          .size(AvatarSize::Sm)
          .id("small"),
      )
      .child(
        AvatarGroup::new(["Ivan Zaida", "Anna K."].map(Avatar::new))
          .max(5)
          .id("all"),
      )
      .into()
  });
  let (.., width, height) = stage.bounds("small");
  assert_eq!((width, height), (42.0, 24.0), "two of 24, 6 over each other");
  assert_eq!(stage.text("IZ").1, 10.0);
  let (.., width, _) = stage.bounds("all");
  assert_eq!(width, 56.0, "a most above how many there are leaves nobody out");
  assert!(!stage.paints("+0"));
}

#[test]
fn a_theme_restyles_an_avatar_or_gives_it_another_anatomy() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Avatar>(|recipe| recipe.base(AvatarPart::Face, |s| s.background(PaletteColor::Primary)));
  let mut stage = Stage::in_app(app, |_| {
    Row::new()
      .padding(8.0)
      .child(Avatar::new("Ivan Zaida").id("ivan"))
      .into()
  });
  assert_eq!(stage.fill_color("ivan"), Some(light(PaletteColor::Primary)));

  let app = App::new();
  // The whole name beside the face, in place of a face alone.
  let renderer: AvatarRenderer = Arc::new(|view: AvatarView| {
    let name = view.part(AvatarPart::Initials).text(&view.name);
    Row::new()
      .spacing(8.0)
      .child(view.face().child(view.fallback()))
      .child(name)
      .into()
  });
  app.theme().set_renderer::<Avatar>(renderer);
  let stage = Stage::in_app(app, |_| Row::new().padding(8.0).child(Avatar::new("Ivan Zaida")).into());
  assert!(stage.paints("IZ"));
  assert!(stage.paints("Ivan Zaida"));
}

/// A picture of 4 by 4 in one color.
#[cfg(feature = "raster")]
fn pixels() -> lurq::components::Image {
  let rgba = [30, 120, 200, 255].repeat(16);
  lurq::components::Image::new(lurq::images::ImageData::from_rgba(rgba, 4, 4))
}

/// The image the avatar with `id` paints, if it paints one in its box.
#[cfg(feature = "raster")]
fn picture(stage: &mut Stage, id: &str) -> Option<Quad> {
  let (x, y, width, height) = stage.bounds(id);
  stage.quads().into_iter().find(|quad| {
    matches!(&quad.content, QuadContent::Image { .. })
      && (quad.x, quad.y, quad.width, quad.height) == (x, y, width, height)
  })
}

#[cfg(feature = "raster")]
#[test]
fn a_picture_fills_the_face_in_its_shape_in_place_of_the_letters() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Avatar::new("Ivan Zaida").image(pixels()).id("round"))
      .child(
        Avatar::new("Lurq Team")
          .image(pixels())
          .shape(AvatarShape::Square)
          .size(AvatarSize::Lg)
          .id("square"),
      )
      .into()
  });
  let round = picture(&mut stage, "round").expect("the picture fills the face");
  assert_eq!(corner(&round), 999.0, "clipped to a circle");
  let square = picture(&mut stage, "square").expect("the picture fills the face");
  assert_eq!((square.width, square.height), (40.0, 40.0));
  assert_eq!(corner(&square), 6.0, "clipped to the square's corners");
  assert!(
    !stage.paints("IZ") && !stage.paints("LT"),
    "the picture is in the place of the letters"
  );
}

#[cfg(feature = "image")]
#[test]
fn a_picture_that_decodes_shows_and_one_that_fails_leaves_the_letters() {
  use std::time::Duration;

  use lurq::{
    components::Image,
    images::{ImageKey, wait_for_decoded_images},
  };

  const RED: &[u8] = include_bytes!("../assets/images/red_64x32.png");
  // Keys no other test uses: the cache of decoded pictures is the process's.
  let red = || Image::from_bytes(RED).cache_key(ImageKey::new(0xA7A7_0001));
  let broken = || Image::from_bytes(&b"not a picture"[..]).cache_key(ImageKey::new(0xA7A7_0002));
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Avatar::new("Ivan Zaida").image(red()).id("decodes"))
      .child(Avatar::new("Anna K.").image(broken()).id("fails"))
      .into()
  });
  assert!(wait_for_decoded_images(Duration::from_secs(10)), "the decodes end");
  stage.pass();

  let shown = picture(&mut stage, "decodes").expect("the decoded picture fills the face");
  assert_eq!(corner(&shown), 999.0, "clipped to a circle");
  assert!(!stage.paints("IZ"), "the letters are gone once the picture shows");
  assert!(picture(&mut stage, "fails").is_none());
  assert_eq!(
    stage.text("AK"),
    (light(PaletteColor::Foreground), 12.0),
    "a picture that fails leaves the letters"
  );
  assert_eq!(stage.fill_color("fails"), Some(light(PaletteColor::Muted)));
}
