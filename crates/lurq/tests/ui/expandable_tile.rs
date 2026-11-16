//! `ui::ExpandableTile`: the design's tile (`slE1t`), what a press of it does, and what it holds.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Text},
  core::Signal,
  layout::{quad::QuadContent, text_style::FontWeight},
  node::color::Color,
  ui::{Button, ExpandableTile, ExpandableTilePart, Interaction, icons},
};

use super::stage::Stage;

const TITLE: &str = "Renderer";
const DETAIL: &str = "wgpu 26 on DirectX 12. Frame time 6.9 ms median.";
const CHEVRON_UP: &str = "\u{E01C}";
const CHEVRON_DOWN: &str = "\u{E019}";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn staged(scene: impl Fn() -> ExpandableTile + Send + Sync + 'static) -> Stage {
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Light));
  Stage::in_app(app, move |_| {
    Column::new().size(400.0, 300.0).padding(8.0).child(scene()).into()
  })
}

/// The glyphs of the icon font that are painted.
fn glyphs(stage: &Stage) -> Vec<String> {
  let glyph = |c: char| ('\u{E000}'..='\u{F8FF}').contains(&c);
  stage
    .quads()
    .iter()
    .filter_map(|quad| match &quad.content {
      QuadContent::Text { text, .. } if !text.is_empty() && text.chars().all(glyph) => Some(text.clone()),
      _ => None,
    })
    .collect()
}

#[test]
fn the_glyphs_are_the_chevrons() {
  let of = |name: &str| {
    icons::ALL
      .iter()
      .find(|(known, _)| *known == name)
      .map(|(_, glyph)| glyph.to_string())
      .expect("an icon of the set")
  };
  assert_eq!(of(icons::CHEVRON_UP), CHEVRON_UP);
  assert_eq!(of(icons::CHEVRON_DOWN), CHEVRON_DOWN);
}

#[test]
fn the_open_tile_is_the_boards() {
  let mut stage = staged(|| ExpandableTile::text(TITLE, DETAIL).open(true).id("tile"));
  let (x, y, width, height) = stage.bounds("tile");
  assert_eq!((x, y, width), (8.0, 8.0, 260.0));
  // 16, a title of 17, 10, two lines of 19.5, 16: 99 on the board.
  assert!((height - 99.0).abs() < 1.5, "{height}");
  assert_eq!(stage.fill_color("tile"), Some(light(PaletteColor::Background)));
  let edge = stage.border("tile").and_then(|quad| quad.border).expect("its edge");
  let top = edge.top.expect("an edge on every side");
  assert_eq!((top.width, top.color), (1.0, light(PaletteColor::Border)));
  // The title: 14 at medium weight in Foreground, 16 in.
  let title = stage.text_quad(TITLE).expect("the title");
  assert_eq!((title.x, title.y), (x + 16.0, y + 16.0));
  match &title.content {
    QuadContent::Text { style, .. } => {
      assert_eq!((style.font_size, style.weight), (14.0, FontWeight::Medium));
      assert_eq!(style.color, light(PaletteColor::Foreground));
    }
    _ => unreachable!(),
  }
  // The chevron: `chevron-up` of 16 in MutedForeground at the end of the header, in the middle of the title's line.
  let chevron = stage.text_quad(CHEVRON_UP).expect("the chevron of the open tile");
  assert!((chevron.x + chevron.width - (x + width - 16.0)).abs() < 0.01);
  assert!((chevron.y + chevron.height / 2.0 - (title.y + title.height / 2.0)).abs() < 0.51);
  assert_eq!(stage.text(CHEVRON_UP), (light(PaletteColor::MutedForeground), 16.0));
  // What it opens: 13 on lines of 1.5 in MutedForeground, 10 under the header, wrapping at the tile's room.
  let detail = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text.starts_with("wgpu 26")))
    .expect("the detail");
  assert_eq!(detail.x, x + 16.0);
  assert!((detail.y - (title.y + title.height + 10.0)).abs() < 0.01);
  assert!(detail.width <= 228.0 + 0.01 && detail.height > 30.0, "it wraps");
  match &detail.content {
    QuadContent::Text { style, .. } => {
      assert_eq!((style.font_size, style.line_height), (13.0, 1.5));
      assert_eq!(style.color, light(PaletteColor::MutedForeground));
    }
    _ => unreachable!(),
  }
}

#[test]
fn closed_it_is_its_header_alone() {
  let mut stage = staged(|| ExpandableTile::text(TITLE, DETAIL).id("tile"));
  let (.., height) = stage.bounds("tile");
  // 16, the title, 16: the room what it opens leaves is closed too.
  assert!((height - 49.0).abs() < 1.5, "{height}");
  assert!(
    !stage
      .quads()
      .iter()
      .any(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text.starts_with("wgpu")))
  );
  assert_eq!(glyphs(&stage), [CHEVRON_DOWN]);
}

#[test]
fn a_press_turns_a_bound_tile_and_tells_its_handlers_after() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let (tell, open) = (told.clone(), Signal::new(false));
  let read = open.clone();
  let mut stage = staged(move || {
    let (tell, seen) = (tell.clone(), read.clone());
    ExpandableTile::text(TITLE, DETAIL)
      .on_open_change(move |next| tell.lock().unwrap().push((next, seen.get_untracked())))
      .bind(read.clone())
      .id("tile")
  });
  stage.click("tile");
  assert!(open.get(), "a press anywhere on the tile opens it");
  assert_eq!(*told.lock().unwrap(), [(true, true)], "told once the signal is set");
  assert_eq!(glyphs(&stage), [CHEVRON_UP]);
  // The keyboard: Tab reaches it, Enter and Space press it.
  stage.key("Enter");
  assert!(!open.get());
  stage.key(" ");
  assert!(open.get());
  assert_eq!(told.lock().unwrap().len(), 3);
  // The app sets the signal: it closes, and nothing is told.
  open.set(false);
  stage.pass();
  assert_eq!(glyphs(&stage), [CHEVRON_DOWN]);
  assert_eq!(told.lock().unwrap().len(), 3);
}

#[test]
fn it_opens_with_its_height() {
  let open = Signal::new(false);
  let read = open.clone();
  let mut stage = staged(move || ExpandableTile::text(TITLE, DETAIL).bind(read.clone()).id("tile"));
  let (.., closed) = stage.bounds("tile");
  open.set(true);
  // One frame after the change it is on its way: higher than closed, lower than open.
  stage.frame();
  stage.frame();
  let (.., moving) = stage.bounds("tile");
  stage.pass();
  let (.., opened) = stage.bounds("tile");
  assert!(closed < moving && moving < opened, "{closed} < {moving} < {opened}");
}

#[test]
fn a_disabled_tile_takes_nothing() {
  let open = Signal::new(false);
  let read = open.clone();
  let mut stage = staged(move || {
    ExpandableTile::text(TITLE, DETAIL)
      .bind(read.clone())
      .disabled(true)
      .id("tile")
  });
  stage.click("tile");
  stage.key("Tab");
  stage.key("Enter");
  assert!(!open.get());
}

#[test]
fn the_focus_is_a_ring_around_the_tile_and_the_pointer_colors_its_edge() {
  let mut stage = staged(|| ExpandableTile::text(TITLE, DETAIL).id("tile"));
  let ring = light(PaletteColor::Ring);
  assert!(!stage.has_ring(ring));
  stage.key("Tab");
  assert!(stage.has_ring(ring), "Tab stops on the tile");
  let mut forced = staged(|| {
    ExpandableTile::text(TITLE, DETAIL)
      .force(Some(Interaction::Hovered))
      .id("tile")
  });
  let top = forced
    .border("tile")
    .and_then(|quad| quad.border)
    .and_then(|border| border.top)
    .expect("its edge");
  assert_eq!(top.color, light(PaletteColor::Ring));
}

#[test]
fn what_the_app_puts_in_it_keeps_its_presses_and_is_a_stop_of_its_own() {
  let pressed = Arc::new(Mutex::new(0));
  let (count, open) = (pressed.clone(), Signal::new(true));
  let read = open.clone();
  let mut stage = staged(move || {
    let count = count.clone();
    ExpandableTile::new(
      "Team",
      Button::new("Compare plans")
        .id("compare")
        .on_click(move |_| *count.lock().unwrap() += 1),
    )
    .child(Text::new("$12 a seat").id("price"))
    .bind(read.clone())
    .id("tile")
  });
  // What it always shows is 10 under the header, and stays when it closes.
  let title = stage.text_quad("Team").expect("the title");
  let (_, price_y, ..) = stage.bounds("price");
  assert!((price_y - (title.y + title.height + 10.0)).abs() < 0.01);
  // A press of the app's button is the button's alone.
  stage.click("compare");
  assert_eq!(*pressed.lock().unwrap(), 1);
  assert!(open.get(), "the tile did not close");
  // So is a press of what it always shows.
  stage.click("price");
  assert!(open.get());
  // Tab goes from the tile to the button in it: the tile has the focus from a press of its header.
  let (x, y, ..) = stage.bounds("tile");
  stage.tree.mouse_move(x + 20.0, y + 20.0);
  stage.tree.mouse_down(x + 20.0, y + 20.0, MouseButton::Left);
  stage.tree.mouse_up(x + 20.0, y + 20.0, MouseButton::Left);
  stage.pass();
  assert!(!open.get());
  stage.key("Enter");
  assert!(open.get(), "Enter on the tile opens it again");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(
    *pressed.lock().unwrap(),
    2,
    "the button in it is the next stop, and Enter is its own"
  );
  assert!(open.get());
  // Closed, what it always shows is there and the button is not.
  open.set(false);
  stage.pass();
  assert!(stage.paints("$12 a seat"));
  assert!(!stage.paints("Compare plans"));
}

#[test]
fn a_long_title_wraps_and_keeps_the_chevron_on_its_first_line() {
  const LONG: &str = "The renderer this window was opened with on the second monitor";
  let mut stage = staged(|| ExpandableTile::text(LONG, DETAIL).id("tile"));
  let (x, y, width, height) = stage.bounds("tile");
  assert_eq!(width, 260.0);
  assert!(height > 60.0, "{height}: the title goes on to a second line");
  let chevron = stage.text_quad(CHEVRON_DOWN).expect("the chevron");
  assert!(
    (chevron.x + chevron.width - (x + width - 16.0)).abs() < 0.01,
    "it keeps its room"
  );
  assert!(chevron.y < y + 16.0 + 17.0, "on the first line");
  let title = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text.starts_with("The renderer")))
    .expect("the title");
  assert!(title.x + title.width <= chevron.x - 8.0 + 0.01, "8 before the chevron");
}

#[test]
fn the_chevron_has_the_icon_it_is_given_and_none_with_none() {
  let shown = |open: bool, tile: fn(ExpandableTile) -> ExpandableTile| {
    glyphs(&staged(move || tile(ExpandableTile::text(TITLE, DETAIL).open(open))))
  };
  let glyph = |name: &str| {
    icons::ALL
      .iter()
      .find(|(known, _)| *known == name)
      .map(|(_, glyph)| glyph.to_string())
      .unwrap()
  };
  assert_eq!(shown(false, |t| t.icon(icons::PLUS)), [glyph(icons::PLUS)]);
  // The open form has the closed one's icon where it is given none of its own.
  assert_eq!(shown(true, |t| t.icon(icons::PLUS)), [glyph(icons::PLUS)]);
  assert_eq!(
    shown(true, |t| t.icon(icons::PLUS).open_icon(icons::MINUS)),
    [glyph(icons::MINUS)]
  );
  assert!(shown(false, |t| t.icon(None)).is_empty());
  assert!(shown(true, |t| t.open_icon(None)).is_empty());
}

#[test]
fn it_fills_its_room_when_told_and_a_theme_draws_it_otherwise() {
  let mut stage = staged(|| {
    ExpandableTile::text(TITLE, DETAIL)
      .full_width(true)
      .style(ExpandableTilePart::Title, |s| s.foreground(PaletteColor::Primary))
      .id("tile")
  });
  let (_, _, width, _) = stage.bounds("tile");
  assert_eq!(width, 384.0, "the stage less its padding");
  assert_eq!(stage.text(TITLE).0, light(PaletteColor::Primary));
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn to_tooling_it_is_a_button_that_says_whether_it_is_open() {
  let mut stage = staged(|| ExpandableTile::text(TITLE, DETAIL).open(true).id("tile"));
  let mut said = |stage: &mut Stage, key: &str| {
    stage
      .tree
      .get_element_by_id_mut("tile")
      .and_then(|element| element.description(key))
  };
  assert_eq!(said(&mut stage, "a11y_role").as_deref(), Some("button"));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some(TITLE));
  assert_eq!(said(&mut stage, "a11y_expanded").as_deref(), Some("true"));
  let mut stage = staged(|| ExpandableTile::text(TITLE, DETAIL).label("Renderer details").id("tile"));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Renderer details"));
  assert_eq!(said(&mut stage, "a11y_expanded").as_deref(), Some("false"));
}
