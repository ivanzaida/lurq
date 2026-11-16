//! A persona: the avatar, the name and the line under it against the board,
//! the dot of presence, the sizes, a name longer than the room, an avatar of
//! the app's own, what tooling reads, that it takes no focus, and a theme's
//! rules and anatomy.

use std::sync::Arc;

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{
    Avatar, AvatarPresence, AvatarShape, Button, Persona, PersonaPart, PersonaRenderer, PersonaSize, PersonaView, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The filled square of `side` in `color` inside the box of `id`, with room of `slack` around it: a face, or a
/// dot that reaches past the face by its ring.
fn square(stage: &mut Stage, id: &str, side: f32, color: Color) -> Option<Quad> {
  let (x, y, width, height) = stage.bounds(id);
  stage.quads().into_iter().find(|quad| {
    matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
      && (quad.width, quad.height) == (side, side)
      && quad.x >= x - 0.01
      && quad.y >= y - 0.01
      && quad.x + quad.width <= x + width + 0.01
      && quad.y + quad.height <= y + height + 0.01
  })
}

fn ivan() -> Persona {
  Persona::new("Ivan Zaida").description("Available · Kyiv")
}

#[test]
fn a_persona_is_an_avatar_10_before_a_name_over_a_line_in_the_muted_color() {
  let mut stage = Stage::new(|_| Column::new().padding(24.0).child(ivan().id("ivan")).into());
  let (x, y, _, height) = stage.bounds("ivan");
  let face = square(&mut stage, "ivan", 32.0, light(PaletteColor::Muted)).expect("an avatar of 32");
  assert_eq!(face.x, x, "the avatar is first");
  assert_eq!(stage.text("IZ").1, 12.0, "the initials of the name");

  assert_eq!(stage.text("Ivan Zaida"), (light(PaletteColor::Foreground), 14.0));
  assert_eq!(
    stage.text("Available · Kyiv"),
    (light(PaletteColor::MutedForeground), 12.0)
  );
  let name = stage.text_quad("Ivan Zaida").unwrap();
  let line = stage.text_quad("Available · Kyiv").unwrap();
  assert_eq!(name.x - (face.x + face.width), 10.0, "10 after the avatar");
  assert_eq!(line.x, name.x, "the line starts where the name does");
  assert!(
    (line.y - (name.y + name.height) - 1.0).abs() < 0.01,
    "1 under the name: {}",
    line.y - (name.y + name.height)
  );
  // The avatar and the text are level about one middle.
  let text_middle = (name.y + line.y + line.height) / 2.0;
  assert!((text_middle - (y + height / 2.0)).abs() < 0.51);
  assert!((face.y + 16.0 - (y + height / 2.0)).abs() < 0.51);
}

#[test]
fn a_persona_without_a_description_is_the_name_alone_beside_the_avatar() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(24.0)
      .child(Persona::new("Ivy Chen").id("ivy"))
      .into()
  });
  let (_, _, _, height) = stage.bounds("ivy");
  assert_eq!(height, 32.0, "as high as its avatar");
  assert!(stage.paints("Ivy Chen"));
}

#[test]
fn the_dot_of_presence_is_at_the_corner_of_the_avatar_in_the_color_of_the_presence() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(24.0)
      .spacing(16.0)
      .child(ivan().presence(AvatarPresence::Online).id("online"))
      .child(ivan().presence(AvatarPresence::Away).id("away"))
      .child(ivan().id("none"))
      .into()
  });
  for (id, role) in [("online", PaletteColor::Success), ("away", PaletteColor::Warning)] {
    let face = square(&mut stage, id, 32.0, light(PaletteColor::Muted)).unwrap();
    let dot = square(&mut stage, id, 10.0, light(role)).unwrap_or_else(|| panic!("{id} has its dot"));
    assert_eq!(
      (dot.x + dot.width, dot.y + dot.height),
      (face.x + face.width, face.y + face.height),
      "{id}: at the bottom end of the face"
    );
  }
  assert!(square(&mut stage, "none", 10.0, light(PaletteColor::Success)).is_none());
}

#[test]
fn the_sizes_follow_the_avatars_with_the_text_to_match() {
  let sizes = [
    (PersonaSize::Xs, "xs", 20.0, 6.0, 12.0, 11.0),
    (PersonaSize::Sm, "sm", 24.0, 8.0, 13.0, 11.0),
    (PersonaSize::Md, "md", 32.0, 10.0, 14.0, 12.0),
    (PersonaSize::Lg, "lg", 40.0, 12.0, 15.0, 13.0),
    (PersonaSize::Xl, "xl", 56.0, 14.0, 18.0, 14.0),
  ];
  for (size, id, side, gap, name_size, line_size) in sizes {
    let mut stage = Stage::new(move |_| {
      Column::new()
        .padding(24.0)
        .child(Persona::new("Mark Rossi").description("Reviewer").size(size).id(id))
        .into()
    });
    let face = square(&mut stage, id, side, light(PaletteColor::Muted)).unwrap_or_else(|| panic!("{id}: {side}"));
    let name = stage.text_quad("Mark Rossi").unwrap();
    assert_eq!(name.x - (face.x + face.width), gap, "{id}: the room after the avatar");
    assert_eq!(stage.text("Mark Rossi").1, name_size, "{id}");
    assert_eq!(stage.text("Reviewer").1, line_size, "{id}");
  }
}

#[test]
fn a_name_longer_than_the_room_ends_in_an_ellipsis_and_the_avatar_keeps_its_size() {
  const NAME: &str = "Maximiliana Alexandrovna von Hohenzollern-Sigmaringen";
  const LINE: &str = "Away until the end of the next sprint, back on the first Monday";
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new()
          .id("room")
          .width(160.0)
          .child(Persona::new(NAME).description(LINE).id("long")),
      )
      .into()
  });
  let (room, _, room_width, _) = stage.bounds("room");
  let (x, _, width, _) = stage.bounds("long");
  assert!(
    x >= room && x + width <= room + room_width + 0.5,
    "no wider than its room: {width}"
  );
  assert!(square(&mut stage, "long", 32.0, light(PaletteColor::Muted)).is_some());
  for words in [NAME, LINE] {
    assert!(stage.text_quad(words).is_none(), "not drawn whole");
    let cut = stage.cut_text_quad(words).expect("cut with an ellipsis");
    assert!(
      cut.x + cut.width <= room + room_width + 0.5,
      "{words:?} ends in the room: {} at {}",
      cut.width,
      cut.x
    );
  }
}

#[test]
fn an_avatar_of_the_apps_own_takes_the_personas_size_and_presence() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(24.0)
      .child(
        Persona::new("Build bot")
          .description("Runs on every push")
          .avatar(
            Avatar::new("Build bot")
              .icon(icons::TERMINAL)
              .shape(AvatarShape::Square),
          )
          .presence(AvatarPresence::Busy)
          .size(PersonaSize::Lg)
          .id("bot"),
      )
      .into()
  });
  let face = square(&mut stage, "bot", 40.0, light(PaletteColor::Muted)).expect("the persona's size");
  assert!(
    face.border_radius.is_some_and(|radius| radius.top_left < 20.0),
    "it keeps its own shape"
  );
  assert!(!stage.paints("BB"), "its icon stands in place of the initials");
  assert!(square(&mut stage, "bot", 12.0, light(PaletteColor::Destructive)).is_some());
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_a_group_called_by_the_name_and_described_by_the_line() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(24.0)
      .child(
        Persona::new("Ivy Chen")
          .description("Away · back at 15:00")
          .presence(AvatarPresence::Away)
          .presence_label("away until 15:00")
          .avatar(Avatar::new("Ivy Chen").id("face"))
          .id("ivy"),
      )
      .into()
  });
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "ivy", "a11y_role").as_deref(), Some("group"));
  assert_eq!(said(&mut stage, "ivy", "a11y_name").as_deref(), Some("Ivy Chen"));
  assert_eq!(
    said(&mut stage, "ivy", "a11y_description").as_deref(),
    Some("Away · back at 15:00")
  );
  assert_eq!(
    said(&mut stage, "face", "a11y_name").as_deref(),
    Some("Ivy Chen, away until 15:00"),
    "the avatar says the presence as the app words it"
  );
}

#[test]
fn a_persona_takes_no_focus_and_tab_passes_it_by() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(24.0)
      .spacing(12.0)
      .child(ivan().presence(AvatarPresence::Online).id("ivan"))
      .child(Button::new("Message").id("message"))
      .into()
  });
  stage.key("Tab");
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some("message"));
}

#[test]
fn a_theme_restyles_a_persona_or_gives_it_another_anatomy() {
  let app = App::new();
  app.theme().patch_recipe::<Persona>(|rules| {
    rules
      .base(PersonaPart::Root, |s| s.gap(20.0))
      .base(PersonaPart::Name, |s| s.foreground(PaletteColor::Destructive))
      .base(PersonaPart::Avatar, |s| s.background(PaletteColor::Secondary))
  });
  let mut stage = Stage::in_app(app, |_| Column::new().padding(24.0).child(ivan().id("ivan")).into());
  let face = square(&mut stage, "ivan", 32.0, light(PaletteColor::Secondary)).expect("the avatar's face restyled");
  let name = stage.text_quad("Ivan Zaida").unwrap();
  assert_eq!(name.x - (face.x + face.width), 20.0);
  assert_eq!(stage.text("Ivan Zaida").0, light(PaletteColor::Destructive));

  let app = App::new();
  let renderer: PersonaRenderer = Arc::new(|view: PersonaView| {
    // The name alone, with no avatar.
    view.root().child(view.text()).into()
  });
  app.theme().set_renderer::<Persona>(renderer);
  let stage = Stage::in_app(app, |_| Column::new().padding(24.0).child(ivan()).into());
  assert!(stage.paints("Ivan Zaida"));
  assert!(!stage.paints("IZ"), "the theme's persona draws no avatar");
}
