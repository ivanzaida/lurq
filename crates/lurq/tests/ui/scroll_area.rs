//! A scroll area: its bar against the board, the ways it scrolls, a gutter, when its bars show, the keys of one
//! with a label, what tooling reads, and a theme's rules and renderer.

use std::sync::Arc;

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Text},
  layout::{layout_kind::ScrollState, quad::QuadContent},
  node::{color::Color, dimension::Dimension},
  ui::{ScrollArea, ScrollAreaOrientation, ScrollAreaPart, ScrollAreaRenderer, ScrollAreaScrollbars, ScrollAreaView},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// What scrolls: a column `width` wide and `height` high.
fn content(width: f32, height: f32) -> Column {
  Column::new().id("content").size(width, height)
}

/// `area` in a window of 400 by 300.
fn stage(area: impl Fn() -> ScrollArea + Send + Sync + 'static) -> Stage {
  Stage::new(move |_| Column::new().size(400.0, 300.0).child(area()).into())
}

/// The thumbs painted: the vertical one (6 across, longer than it is wide) and the horizontal one.
fn thumbs(stage: &Stage) -> (Option<(f32, f32, f32, f32)>, Option<(f32, f32, f32, f32)>) {
  let rects: Vec<_> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Ring)
        || *color == light(PaletteColor::MutedForeground))
    })
    .map(|quad| (quad.x, quad.y, quad.width, quad.height))
    .collect();
  let vertical = rects.iter().copied().find(|(_, _, w, h)| *w == 6.0 && *h > 12.0);
  let horizontal = rects.iter().copied().find(|(_, _, w, h)| *h == 6.0 && *w > 12.0);
  (vertical, horizontal)
}

#[test]
fn its_bar_is_the_boards_a_lane_of_8_with_a_round_thumb_of_6_in_the_ring_color() {
  let mut stage = stage(|| {
    ScrollArea::new()
      .id("area")
      .size_for_test()
      .child(content(200.0, 300.0))
  });
  let (x, y, width, _) = stage.bounds("area");
  let (vertical, horizontal) = thumbs(&stage);
  let (thumb_x, thumb_y, thumb_width, thumb_height) = vertical.expect("a vertical thumb");
  // The lane is 8, 2 from the edge; the thumb is 1 inside it, and starts 1 inside the track's top.
  assert_eq!(thumb_width, 6.0);
  assert!((thumb_x - (x + width - 2.0 - 8.0 + 1.0)).abs() < 0.01, "{thumb_x}");
  assert!((thumb_y - (y + 2.0 + 1.0)).abs() < 0.01, "{thumb_y}");
  // 100 of 300 shows: the thumb is a third of the track less its insets.
  assert!((thumb_height - (100.0 - 4.0 - 2.0) / 3.0).abs() < 0.5, "{thumb_height}");
  assert!(horizontal.is_none(), "it scrolls down only");
  let thumb = stage
    .quads()
    .into_iter()
    .find(|quad| quad.width == 6.0 && quad.height == thumb_height)
    .unwrap();
  assert_eq!(thumb.border_radius.map(|radius| radius.top_left), Some(3.0), "round");
}

#[test]
fn the_thumb_under_the_pointer_is_in_the_muted_foreground() {
  let mut stage = stage(|| {
    ScrollArea::new()
      .id("area")
      .size_for_test()
      .child(content(200.0, 400.0))
  });
  let (x, y, width, height) = thumbs(&stage).0.unwrap();
  stage.tree.mouse_move(x + width / 2.0, y + height / 2.0);
  stage.pass();
  let hovered = stage.quads().into_iter().any(|quad| {
    matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::MutedForeground))
      && quad.width == 6.0
  });
  assert!(hovered, "the thumb under the pointer");
}

#[test]
fn it_scrolls_across_or_both_ways_with_a_bar_at_the_bottom() {
  let across = stage(|| {
    ScrollArea::new()
      .orientation(lurq::ui::Orientation::Horizontal)
      .size_for_test()
      .child(content(800.0, 50.0))
  });
  let (vertical, horizontal) = thumbs(&across);
  assert!(vertical.is_none() && horizontal.is_some(), "across only");
  let (_, y, _, height) = horizontal.unwrap();
  assert_eq!(height, 6.0);
  assert!(
    (y + height - (100.0 - 2.0 - 1.0)).abs() < 0.01,
    "at the bottom, in its lane: {y}"
  );

  let both = stage(|| {
    ScrollArea::new()
      .orientation(ScrollAreaOrientation::Both)
      .size_for_test()
      .child(content(800.0, 400.0))
  });
  let (vertical, horizontal) = thumbs(&both);
  assert!(vertical.is_some() && horizontal.is_some(), "both bars");
}

#[test]
fn a_gutter_keeps_the_bar_beside_the_content() {
  let fill = || Column::new().id("row").width(Dimension::Pct(100.0)).height(400.0);
  let mut over = stage(move || ScrollArea::new().size_for_test().child(fill()));
  assert_eq!(over.bounds("row").2, 200.0, "over the content's edge");
  let mut beside = stage(move || ScrollArea::new().gutter(true).size_for_test().child(fill()));
  // The lane of 8 and its 2 on either side.
  assert_eq!(beside.bounds("row").2, 188.0, "beside it");
}

#[test]
fn its_bars_show_as_it_says() {
  let never = stage(|| {
    ScrollArea::new()
      .scrollbars(ScrollAreaScrollbars::Never)
      .size_for_test()
      .child(content(200.0, 400.0))
  });
  assert!(thumbs(&never).0.is_none(), "never");

  let always = stage(|| {
    ScrollArea::new()
      .scrollbars(ScrollAreaScrollbars::Always)
      .size_for_test()
      .child(content(200.0, 50.0))
  });
  assert!(thumbs(&always).0.is_some(), "always, though it fits");

  let auto = stage(|| ScrollArea::new().size_for_test().child(content(200.0, 50.0)));
  assert!(thumbs(&auto).0.is_none(), "none while it fits");

  let mut hover = stage(|| {
    ScrollArea::new()
      .id("area")
      .scrollbars(ScrollAreaScrollbars::Hover)
      .size_for_test()
      .child(content(200.0, 400.0))
  });
  assert!(thumbs(&hover).0.is_none(), "hidden at rest");
  hover.hover("area");
  assert!(thumbs(&hover).0.is_some(), "the pointer over it shows it");

  let mut scrolling = stage(|| {
    ScrollArea::new()
      .id("area")
      .scrollbars(ScrollAreaScrollbars::Scrolling)
      .size_for_test()
      .child(content(200.0, 400.0))
  });
  scrolling.hover("area");
  assert!(thumbs(&scrolling).0.is_none(), "not for the pointer alone");
}

#[test]
fn a_labelled_area_is_a_stop_of_tab_whose_keys_scroll_it_with_the_ring_inside() {
  let state = ScrollState::new();
  let held = state.clone();
  let mut stage = stage(move || {
    ScrollArea::new()
      .id("area")
      .label("Terms")
      .scroll_state(held.clone())
      .size_for_test()
      .child(Text::new("A long text that has nothing that takes the focus."))
      .child(content(200.0, 400.0))
  });
  stage.key("Tab");
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some("area"));
  assert!(stage.has_ring(light(PaletteColor::Ring)), "its ring");
  stage.key("ArrowDown");
  assert_eq!(state.scroll_y(), 40.0);
  stage.key("End");
  assert_eq!(state.scroll_y(), state.content_height() - state.viewport_height());

  // One with no label is no stop of Tab: its content's controls are.
  let mut unnamed = super::scroll_area::stage(|| {
    ScrollArea::new()
      .id("area")
      .size_for_test()
      .child(content(200.0, 400.0))
  });
  unnamed.key("Tab");
  assert!(unnamed.tree.focused_element().is_none());
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn a_labelled_area_is_a_region_of_its_name() {
  let mut stage = stage(|| {
    ScrollArea::new()
      .id("area")
      .label("Terms")
      .size_for_test()
      .child(content(200.0, 400.0))
  });
  let said = |stage: &mut Stage, name: &str| {
    stage
      .tree
      .get_element_by_id_mut("area")
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "a11y_role").as_deref(), Some("region"));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Terms"));
}

#[test]
fn a_rule_of_the_theme_changes_the_area_and_a_renderer_draws_another() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<ScrollArea>(|recipe| recipe.base(ScrollAreaPart::Root, |s| s.background(PaletteColor::Muted)));
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        ScrollArea::new()
          .id("area")
          .size_for_test()
          .child(content(200.0, 400.0)),
      )
      .into()
  });
  assert_eq!(stage.fill_color("area"), Some(light(PaletteColor::Muted)));

  let app = App::new();
  let renderer: ScrollAreaRenderer =
    Arc::new(|mut view: ScrollAreaView| Column::new().id("drawn").child(view.content()).into());
  app.theme().set_renderer::<ScrollArea>(renderer);
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(ScrollArea::new().size_for_test().child(content(200.0, 40.0)))
      .into()
  });
  assert_eq!(
    stage.bounds("drawn").3,
    40.0,
    "the theme's anatomy, with what scrolls in it"
  );
}

/// The area of these tests: 200 by 100.
trait SizeForTest {
  fn size_for_test(self) -> Self;
}

impl SizeForTest for ScrollArea {
  fn size_for_test(self) -> Self {
    self.width(200.0).height(100.0)
  }
}

#[test]
fn a_thumb_of_a_palettes_role_is_that_role_in_each_appearance() {
  use lurq::layout::scrollbar::ScrollBarStyle;
  let bar = ScrollBarStyle::default().with_thumb_role(Some(PaletteColor::Border));
  for palette in [ThemePalette::light(), ThemePalette::dark()] {
    let border = palette.get(PaletteColor::Border);
    assert_eq!(bar.thumb_in(&palette), border);
    assert_eq!(
      bar.thumb_when(&palette, true, false),
      border,
      "under the pointer it keeps it, told no other"
    );
  }
  assert_ne!(
    ThemePalette::light().get(PaletteColor::Border),
    ThemePalette::dark().get(PaletteColor::Border),
    "one role, two colors"
  );
  let still = ScrollBarStyle::default();
  assert_eq!(
    still.thumb_in(&ThemePalette::dark()),
    ThemePalette::dark().get(PaletteColor::Ring),
    "the theme's Ring unless told"
  );
}
