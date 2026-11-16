//! A splitter: its panes laid out by their sizes, the line between them, the
//! bar under the pointer and while dragged, a drag that holds the pointer and
//! the cursor, closing and opening a pane, the keys, a double press, the sizes
//! the app gives it and saves, both orientations, one splitter in another, a
//! disabled one, and a theme's other look.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::Column,
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::{CursorIcon, color::Color, dimension::Dimension},
  ui::{Orientation, Splitter, SplitterPane, SplitterPart},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// What a pane holds: a column that fills it, called `id`.
fn content(id: &str) -> Column {
  Column::new()
    .id(id.to_owned())
    .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
}

/// A splitter of the stage's whole room: a side bar of 120 between 80 and 240 that closes, an editor that takes the
/// rest, and a preview of a quarter.
fn three() -> Splitter {
  Splitter::new()
    .id("s")
    .pane(
      SplitterPane::new(content("a"))
        .length(120.0)
        .min(80.0)
        .max(240.0)
        .collapsible(true)
        .label("Files"),
    )
    .pane(SplitterPane::new(content("b")).min(60.0))
    .pane(SplitterPane::new(content("c")).share(0.25).min(50.0))
}

/// The filled rectangles of `color` that are `width` wide.
fn rects(stage: &Stage, color: Color, width: f32) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
        && (quad.width - width).abs() < 0.01
    })
    .collect()
}

fn width(stage: &mut Stage, id: &str) -> f32 {
  stage.bounds(id).2
}

/// Presses at `(x, y)`, drags to `(to, y)` and lets go there.
fn drag(stage: &mut Stage, x: f32, y: f32, to: f32) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_move((x + to) / 2.0, y);
  stage.tree.mouse_move(to, y);
  stage.tree.mouse_up(to, y, MouseButton::Left);
  stage.pass();
}

#[test]
fn lays_out_panes_by_pixels_share_and_the_rest() {
  let mut stage = Stage::new(|_| three().into());
  // 400 wide: 120, then the rest, then a quarter, with a line of 1 between each two.
  assert_eq!(stage.bounds("a"), (0.0, 0.0, 120.0, 300.0));
  assert_eq!(stage.bounds("c").2, 100.0);
  assert_eq!(stage.bounds("c").0, 300.0);
  let b = stage.bounds("b");
  assert_eq!((b.0, b.2), (121.0, 178.0));
}

#[test]
fn draws_a_line_of_one_in_the_color_of_an_edge_between_panes() {
  let mut stage = Stage::new(|_| three().into());
  let lines = rects(&stage, light(PaletteColor::Border), 1.0);
  assert_eq!(lines.len(), 2, "a line between each two panes");
  assert!(lines.iter().any(|line| line.x == 120.0 && line.height == 300.0));
  assert!(lines.iter().any(|line| line.x == 299.0 && line.height == 300.0));
  // Nothing of the bar at rest.
  assert!(rects(&stage, light(PaletteColor::Ring), 3.0).is_empty());
  assert!(
    stage.fill_color("s").is_none(),
    "a splitter that is a layout has no fill of its own"
  );
}

#[test]
fn the_bar_is_three_wide_in_the_ring_color_under_the_pointer_and_primary_while_dragged() {
  let mut stage = Stage::new(|_| three().into());
  // Over the edge of the editor, 4 from the line: the room around the line takes the pointer.
  stage.tree.mouse_move(124.0, 150.0);
  stage.pass();
  let bars = rects(&stage, light(PaletteColor::Ring), 3.0);
  assert_eq!(bars.len(), 1);
  assert_eq!((bars[0].x, bars[0].height), (119.0, 300.0), "centered on the line");
  stage.tree.mouse_down(124.0, 150.0, MouseButton::Left);
  stage.pass();
  assert_eq!(rects(&stage, light(PaletteColor::Primary), 3.0).len(), 1);
}

#[test]
fn a_drag_moves_the_divider_and_holds_the_pointer_and_its_cursor() {
  let renders = Arc::new(AtomicUsize::new(0));
  let sizes = Signal::new(Vec::new());
  let mut stage = Stage::new({
    let (renders, sizes) = (renders.clone(), sizes.clone());
    move |_| {
      renders.fetch_add(1, Ordering::Relaxed);
      Splitter::new()
        .sizes(sizes.clone())
        .pane(SplitterPane::new(content("a")).length(120.0).min(80.0).max(240.0))
        .pane(SplitterPane::new(content("b").cursor(CursorIcon::Text)))
        .into()
    }
  });
  let before = renders.load(Ordering::Relaxed);

  stage.tree.mouse_move(120.5, 150.0);
  assert_eq!(stage.tree.cursor(), CursorIcon::ColResize);
  stage.tree.mouse_down(120.5, 150.0, MouseButton::Left);
  stage.tree.mouse_move(180.5, 150.0);
  stage.pass();
  assert_eq!(width(&mut stage, "a"), 180.0);
  assert_eq!(sizes.get()[0], 180.0);
  // Over the editor, whose cursor is the text's, and out of the window: the drag keeps its own.
  stage.tree.mouse_move(300.0, 150.0);
  assert_eq!(stage.tree.cursor(), CursorIcon::ColResize);
  stage.tree.mouse_leave_window();
  stage.tree.mouse_move(900.0, 150.0);
  assert_eq!(stage.tree.cursor(), CursorIcon::ColResize);
  stage.pass();
  assert_eq!(width(&mut stage, "a"), 240.0, "past the window it stops at its most");
  stage.tree.mouse_move(-50.0, 150.0);
  stage.pass();
  assert_eq!(width(&mut stage, "a"), 80.0, "and at its least the other way");
  stage.tree.mouse_up(-50.0, 150.0, MouseButton::Left);
  stage.tree.mouse_move(300.0, 150.0);
  assert_eq!(
    stage.tree.cursor(),
    CursorIcon::Text,
    "let go, the cursor is what is under it again"
  );

  assert_eq!(width(&mut stage, "b"), 400.0 - 1.0 - 80.0);
  assert_eq!(
    renders.load(Ordering::Relaxed),
    before,
    "what holds the splitter is not rendered again by the drag"
  );
}

#[test]
fn on_resize_while_it_moves_and_on_resize_end_when_it_is_let_go() {
  let moves = Arc::new(AtomicUsize::new(0));
  let ends = Arc::new(parking_lot::Mutex::new(Vec::<Vec<f32>>::new()));
  let mut stage = Stage::new({
    let (moves, ends) = (moves.clone(), ends.clone());
    move |_| {
      let (moves, ends) = (moves.clone(), ends.clone());
      three()
        .on_resize(move |_| {
          moves.fetch_add(1, Ordering::Relaxed);
        })
        .on_resize_end(move |sizes| ends.lock().push(sizes.to_vec()))
        .into()
    }
  });
  drag(&mut stage, 120.5, 150.0, 160.5);
  assert!(moves.load(Ordering::Relaxed) >= 1);
  let ends = ends.lock();
  assert_eq!(ends.len(), 1, "once, when it is let go");
  assert_eq!(ends[0][0], 160.0);
  assert_eq!(ends[0][1], 138.0, "the editor gave way");
  assert_eq!(ends[0][2], 100.0, "the preview did not move");
}

#[test]
fn a_collapsible_pane_closes_under_half_its_least_and_opens_again() {
  let mut stage = Stage::new(|_| three().into());
  drag(&mut stage, 120.5, 150.0, 30.0);
  assert_eq!(width(&mut stage, "a"), 0.0, "closed");
  assert_eq!(width(&mut stage, "b"), 400.0 - 2.0 - 100.0);
  // From the line where it is now, at the start.
  drag(&mut stage, 0.5, 150.0, 60.0);
  assert_eq!(width(&mut stage, "a"), 80.0, "opened at its least");
  drag(&mut stage, 80.5, 150.0, 30.0);
  assert_eq!(width(&mut stage, "a"), 0.0);
  // A double press opens it to what it began with, its two presses at one time however slow the machine.
  stage.presses_at(std::time::Duration::ZERO);
  for _ in 0..2 {
    stage.tree.mouse_down(0.5, 150.0, MouseButton::Left);
    stage.tree.mouse_up(0.5, 150.0, MouseButton::Left);
  }
  stage.pass();
  assert_eq!(width(&mut stage, "a"), 120.0);
}

#[test]
fn a_double_press_puts_the_divider_back_where_it_began() {
  let mut stage = Stage::new(|_| three().into());
  drag(&mut stage, 120.5, 150.0, 200.5);
  assert_eq!(width(&mut stage, "a"), 200.0);
  // The two presses at one time: a double press however slow the machine.
  stage.presses_at(std::time::Duration::ZERO);
  for _ in 0..2 {
    stage.tree.mouse_down(200.5, 150.0, MouseButton::Left);
    stage.tree.mouse_up(200.5, 150.0, MouseButton::Left);
  }
  stage.pass();
  assert_eq!(width(&mut stage, "a"), 120.0);
  assert_eq!(width(&mut stage, "b"), 178.0);
}

#[test]
fn the_keys_move_a_divider_focused_from_the_keyboard() {
  let mut stage = Stage::new(|_| three().into());
  stage.key("Tab");
  // Focused from the keyboard, the bar shows in the ring's color.
  assert_eq!(rects(&stage, light(PaletteColor::Ring), 3.0).len(), 1);
  stage.key("ArrowRight");
  assert_eq!(width(&mut stage, "a"), 130.0, "a step of 10");
  stage
    .tree
    .key_down("ArrowLeft".into(), "ArrowLeft".into(), true, false, false);
  stage
    .tree
    .key_up("ArrowLeft".into(), "ArrowLeft".into(), true, false, false);
  stage.pass();
  assert_eq!(width(&mut stage, "a"), 80.0, "50 with Shift, stopped at its least");
  stage.key("End");
  // Its most is 240, but the editor beside it keeps its least of 60 of the 298 the two share.
  assert_eq!(
    width(&mut stage, "a"),
    238.0,
    "End: its most, as far as the editor lets it"
  );
  stage.key("Home");
  assert_eq!(width(&mut stage, "a"), 80.0, "Home: its least");
  stage.key("Enter");
  assert_eq!(width(&mut stage, "a"), 0.0, "Enter closes the pane that closes");
  stage.key("Enter");
  assert_eq!(width(&mut stage, "a"), 120.0, "and opens it again");
  // The second divider is the next stop of Tab: the editor before it gives way to the preview.
  stage.key("Tab");
  stage.key("ArrowLeft");
  assert_eq!(width(&mut stage, "c"), 110.0);
}

#[test]
fn a_vertical_splitter_lays_its_panes_one_over_the_other() {
  let mut stage = Stage::new(|_| {
    Splitter::new()
      .orientation(Orientation::Vertical)
      .pane(SplitterPane::new(content("top")).min(40.0))
      .pane(SplitterPane::new(content("bottom")).length(100.0).min(40.0))
      .into()
  });
  assert_eq!(stage.bounds("top"), (0.0, 0.0, 400.0, 199.0));
  assert_eq!(stage.bounds("bottom"), (0.0, 200.0, 400.0, 100.0));
  let lines: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Border))
        && quad.height == 1.0
    })
    .collect();
  assert!(lines.iter().any(|line| line.y == 199.0 && line.width == 400.0));
  stage.tree.mouse_move(200.0, 199.5);
  assert_eq!(stage.tree.cursor(), CursorIcon::RowResize);
  stage.tree.mouse_down(200.0, 199.5, MouseButton::Left);
  stage.tree.mouse_move(200.0, 149.5);
  stage.tree.mouse_up(200.0, 149.5, MouseButton::Left);
  stage.pass();
  assert_eq!(stage.bounds("bottom").3, 150.0, "the pane of pixels grew by the drag");
}

#[test]
fn splitters_nest_one_orientation_in_the_other() {
  let mut stage = Stage::new(|_| {
    Splitter::new()
      .pane(SplitterPane::new(content("side")).length(100.0))
      .pane(SplitterPane::new(
        Splitter::new()
          .orientation(Orientation::Vertical)
          .id("inner")
          .pane(SplitterPane::new(content("editor")))
          .pane(SplitterPane::new(content("terminal")).length(80.0)),
      ))
      .into()
  });
  assert_eq!(stage.bounds("inner"), (101.0, 0.0, 299.0, 300.0));
  assert_eq!(stage.bounds("editor"), (101.0, 0.0, 299.0, 219.0));
  assert_eq!(stage.bounds("terminal"), (101.0, 220.0, 299.0, 80.0));
  // The inner divider is dragged by itself.
  stage.tree.mouse_move(250.0, 219.5);
  stage.tree.mouse_down(250.0, 219.5, MouseButton::Left);
  stage.tree.mouse_move(250.0, 199.5);
  stage.tree.mouse_up(250.0, 199.5, MouseButton::Left);
  stage.pass();
  assert_eq!(stage.bounds("terminal").3, 100.0);
  assert_eq!(stage.bounds("side").2, 100.0);
}

#[test]
fn the_sizes_the_app_gives_are_where_it_begins() {
  let sizes = Signal::new(vec![200.0, 98.0, 100.0]);
  let mut stage = Stage::new({
    let sizes = sizes.clone();
    move |_| three().sizes(sizes.clone()).into()
  });
  assert_eq!(width(&mut stage, "a"), 200.0);
  assert_eq!(width(&mut stage, "b"), 98.0);
  assert_eq!(width(&mut stage, "c"), 100.0);
}

#[test]
fn a_mounted_splitter_shows_sizes_the_app_sets_from_outside() {
  let sizes = Signal::new(Vec::new());
  let mut stage = Stage::new({
    let sizes = sizes.clone();
    move |ctx| three().sizes(sizes.clone()).mount(ctx)
  });
  assert_eq!(width(&mut stage, "a"), 120.0);
  assert_eq!(
    sizes.get(),
    vec![120.0, 178.0, 100.0],
    "the sizes are written once it is laid out"
  );
  sizes.set(vec![160.0, 138.0, 100.0]);
  stage.pass();
  assert_eq!(width(&mut stage, "a"), 160.0);
}

#[test]
fn a_mounted_splitter_keeps_its_own_sizes_when_what_holds_it_renders_again() {
  let other = Signal::new(0);
  let mut stage = Stage::new({
    let other = other.clone();
    move |ctx| {
      // Read so that the scene renders again when it changes.
      let _ = other.get();
      three().mount(ctx)
    }
  });
  drag(&mut stage, 120.5, 150.0, 180.5);
  assert_eq!(width(&mut stage, "a"), 180.0);
  other.set(1);
  stage.pass();
  assert_eq!(width(&mut stage, "a"), 180.0);
}

#[test]
fn the_window_shrinking_keeps_a_pane_at_its_least() {
  let room = Signal::new(400.0_f32);
  let mut stage = Stage::new({
    let room = room.clone();
    move |_| {
      Column::new()
        .width(room.get())
        .height(300.0)
        .child(
          Splitter::new()
            .pane(SplitterPane::new(content("a")).length(200.0).min(100.0))
            .pane(SplitterPane::new(content("b")).min(150.0)),
        )
        .into()
    }
  });
  assert_eq!(width(&mut stage, "b"), 199.0);
  room.set(301.0);
  stage.pass();
  // The pane of pixels keeps them while the rest has its least; past that it gives way.
  assert_eq!(width(&mut stage, "b"), 150.0);
  assert_eq!(width(&mut stage, "a"), 150.0);
}

#[test]
fn a_disabled_splitter_takes_no_drag_and_no_focus() {
  let mut stage = Stage::new(|_| three().disabled(true).into());
  drag(&mut stage, 120.5, 150.0, 200.5);
  assert_eq!(width(&mut stage, "a"), 120.0);
  stage.tree.mouse_move(120.5, 150.0);
  stage.pass();
  assert_ne!(stage.tree.cursor(), CursorIcon::ColResize);
  assert!(rects(&stage, light(PaletteColor::Ring), 3.0).is_empty());
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(width(&mut stage, "a"), 120.0);
}

#[test]
fn the_grip_and_the_frame_of_the_board() {
  let mut stage = Stage::new(|_| three().grip(true).framed(true).into());
  // The grip of 12 by 20 in the color of an edge, centered on the first line.
  let grips: Vec<Quad> = rects(&stage, light(PaletteColor::Border), 12.0)
    .into_iter()
    .filter(|quad| quad.height == 20.0)
    .collect();
  assert_eq!(grips.len(), 2, "one on each divider");
  assert!(grips.iter().any(|grip| grip.x == 114.5 && grip.y == 140.0));
  assert_eq!(stage.fill_color("s"), Some(light(PaletteColor::Background)));
  assert!(stage.border("s").is_some(), "the board's edge");
}

#[test]
fn a_theme_restyles_its_parts() {
  let stage = Stage::new(|_| {
    three()
      .style(SplitterPart::Line, |s| s.background(PaletteColor::Primary))
      .into()
  });
  assert_eq!(rects(&stage, light(PaletteColor::Primary), 1.0).len(), 2);
}

#[test]
fn nothing_in_a_closed_pane_is_a_stop_of_tab() {
  let pressed = Arc::new(AtomicUsize::new(0));
  let mut stage = Stage::new({
    let pressed = pressed.clone();
    move |_| {
      let pressed = pressed.clone();
      Splitter::new()
        .pane(
          SplitterPane::new(lurq::ui::Button::new("Inside").on_click(move |_| {
            pressed.fetch_add(1, Ordering::Relaxed);
          }))
          .length(120.0)
          .min(80.0)
          .collapsible(true),
        )
        .pane(SplitterPane::new(content("b")))
        .into()
    }
  });
  // Open, the button in the pane is the first stop.
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(pressed.load(Ordering::Relaxed), 1);
  drag(&mut stage, 120.5, 150.0, 20.0);
  assert_eq!(stage.bounds("b").0, 1.0, "closed");
  // Closed, the divider is the first: Enter opens the pane, and the button is not pressed.
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(pressed.load(Ordering::Relaxed), 1);
  assert_eq!(stage.bounds("b").0, 121.0, "opened again");
}

/// What a divider says to tooling follows a drag and a key at once, in a splitter that is not rendered again.
#[cfg(feature = "mcp")]
#[test]
fn a_divider_tells_tooling_where_it_is_as_it_moves() {
  let mut stage = Stage::new(|_| three().into());
  let said = |stage: &mut Stage, name: &str| {
    stage
      .tree
      .get_element_by_id_mut("s-divider-0")
      .and_then(|divider| divider.description(name))
  };
  assert_eq!(said(&mut stage, "a11y_role").as_deref(), Some("separator"));
  assert_eq!(said(&mut stage, "a11y_orientation").as_deref(), Some("vertical"));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Files"));
  assert_eq!(said(&mut stage, "a11y_valuenow").as_deref(), Some("120"));
  assert_eq!(said(&mut stage, "a11y_valuemin").as_deref(), Some("80"));
  assert_eq!(said(&mut stage, "a11y_valuemax").as_deref(), Some("240"));
  stage.tree.mouse_move(120.5, 150.0);
  stage.tree.mouse_down(120.5, 150.0, MouseButton::Left);
  stage.tree.mouse_move(180.5, 150.0);
  assert_eq!(
    said(&mut stage, "a11y_valuenow").as_deref(),
    Some("180"),
    "while it is dragged"
  );
  stage.tree.mouse_up(180.5, 150.0, MouseButton::Left);
  // A drag focuses nothing; Tab reaches the divider.
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(
    said(&mut stage, "a11y_valuenow").as_deref(),
    Some("190"),
    "and moved by a key"
  );
}

#[test]
fn a_disabled_pane_keeps_its_size_and_the_other_dividers_still_move() {
  let mut stage = Stage::new(|_| {
    Splitter::new()
      .id("s")
      .pane(
        SplitterPane::new(content("a"))
          .length(120.0)
          .min(80.0)
          .collapsible(true)
          .disabled(true)
          .id("pane-a"),
      )
      .pane(SplitterPane::new(content("b")).min(60.0))
      .pane(SplitterPane::new(content("c")).share(0.25).min(50.0))
      .into()
  });
  // The divider after the disabled pane does not move it, by the pointer or by the keys.
  drag(&mut stage, 120.5, 150.0, 200.5);
  assert_eq!(width(&mut stage, "a"), 120.0);
  stage.key("Tab");
  stage.key("ArrowRight");
  stage.key("Enter");
  assert_eq!(width(&mut stage, "a"), 120.0, "nor closes it");
  // The divider between the two others still moves.
  drag(&mut stage, 299.5, 150.0, 279.5);
  assert!((width(&mut stage, "c") - 120.0).abs() < 0.01);
  // The pane's element has its id.
  assert_eq!(stage.bounds("pane-a").2, 120.0);
}

#[test]
fn the_orientation_is_a_state_of_the_recipe() {
  use lurq::ui::{Case, SplitterState};
  let theme = lurq::app::theme::Theme::new();
  let across = theme.resolve::<Splitter>(&Case::new().state(SplitterState::Horizontal, true), None);
  let down = theme.resolve::<Splitter>(&Case::new().state(SplitterState::Vertical, true), None);
  assert_eq!(across.part(SplitterPart::Handle).rest().width, Some(12.0.into()));
  assert_eq!(down.part(SplitterPart::Handle).rest().height, Some(12.0.into()));
}

#[test]
fn what_has_the_focus_in_a_pane_keeps_it_when_what_holds_the_splitter_renders_again() {
  let tick = Signal::new(0);
  let bound = tick.clone();
  let mut stage = Stage::new(move |ctx| {
    let _ = bound.get();
    Column::new()
      .size(600.0, 200.0)
      .child(
        Splitter::new()
          .pane(SplitterPane::new(lurq::components::Button::new("In").id("in").tab_index(0)).share(0.5))
          .pane(SplitterPane::new(Column::new().id("other")))
          .mount(ctx),
      )
      .into()
  });
  stage.tree.resize(600, 200);
  stage.pass();
  stage.click("in");
  let focused = |stage: &Stage| {
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
  };
  assert_eq!(focused(&stage).as_deref(), Some("in"));
  tick.set(1);
  stage.pass();
  assert_eq!(
    focused(&stage).as_deref(),
    Some("in"),
    "the panes are the same nodes after the render"
  );
}

#[test]
fn in_a_column_with_panes_that_fill_themselves_each_divider_stands_at_the_edge_of_its_pane() {
  let mut stage = Stage::new(|ctx| {
    let fill = || content("x").id("inner");
    let inner = Splitter::new()
      .orientation(Orientation::Vertical)
      .pane(SplitterPane::new(fill()).share(0.7))
      .pane(SplitterPane::new(fill()))
      .key("inner")
      .mount(ctx);
    Column::new()
      .size(860.0, 440.0)
      .align_items(lurq::layout::Alignment::Stretch)
      .child(
        Column::new()
          .flex(1.0)
          .align_items(lurq::layout::Alignment::Stretch)
          .child(
            Splitter::new()
              .id("split")
              .pane(SplitterPane::new(content("a")).share(0.25).id("first"))
              .pane(SplitterPane::new(inner))
              .key("outer")
              .mount(ctx),
          ),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(lurq::layout::Constraints::loose(lurq::layout::Size::new(
      860.0, 440.0,
    ))));
  stage.tree.resize(860, 440);
  for _ in 0..3 {
    stage.pass();
  }
  let (first_x, _, first_width, _) = stage.bounds("first");
  let (divider_x, _, divider_width, _) = stage.bounds("split-divider-0");
  assert_eq!(first_width, 215.0, "a quarter of 860");
  assert!(
    ((divider_x + divider_width / 2.0) - (first_x + first_width)).abs() < 1.0,
    "the divider at the pane's edge: {} and {}",
    divider_x + divider_width / 2.0,
    first_x + first_width
  );
}

/// The boxes of the elements with `ids`.
fn boxes(stage: &mut Stage, ids: &[&str]) -> Vec<(f32, f32, f32, f32)> {
  ids.iter().map(|id| stage.bounds(id)).collect()
}

/// How far the middle of the divider `divider` is from the end of the pane `pane`, along a row.
fn off_the_edge(stage: &mut Stage, divider: &str, pane: &str) -> f32 {
  let (x, _, width, _) = stage.bounds(divider);
  let (pane_x, _, pane_width, _) = stage.bounds(pane);
  ((x + width / 2.0) - (pane_x + pane_width)).abs()
}

#[test]
fn the_first_frame_keeps_a_pane_at_its_least_and_no_frame_after_moves_anything() {
  // A quarter of 400 is 100, under the pane's least size of 160.
  let mut stage = Stage::first_frame(400.0, 300.0, |ctx| {
    Splitter::new()
      .id("s")
      .pane(SplitterPane::new(content("a")).share(0.25).min(160.0))
      .pane(SplitterPane::new(content("b")).min(100.0))
      .mount(ctx)
  });
  let ids = ["a", "b", "s-divider-0"];
  let first = boxes(&mut stage, &ids);
  assert_eq!(first[0].2, 160.0, "at its least size in the first frame");
  assert_eq!(first[1].2, 239.0, "the rest beside it");
  assert!(
    off_the_edge(&mut stage, "s-divider-0", "a") < 1.0,
    "the divider at the pane's edge in the first frame"
  );
  for frame in 1..4 {
    stage.frame();
    assert_eq!(boxes(&mut stage, &ids), first, "frame {frame} is the first one");
  }
}

#[test]
fn the_first_frame_has_each_divider_at_the_edge_of_its_pane() {
  // Shares within their limits: nothing is put back, and the dividers still stand where the panes end.
  for vertical in [false, true] {
    let mut stage = Stage::first_frame(400.0, 300.0, move |_| {
      let mut splitter = Splitter::new()
        .id("s")
        .pane(SplitterPane::new(content("a")).share(0.25))
        .pane(SplitterPane::new(content("b")))
        .pane(SplitterPane::new(content("c")).share(0.25));
      if vertical {
        splitter = splitter.orientation(Orientation::Vertical);
      }
      splitter.into()
    });
    let ids = ["a", "b", "c", "s-divider-0", "s-divider-1"];
    let first = boxes(&mut stage, &ids);
    let along = |rect: (f32, f32, f32, f32)| if vertical { (rect.1, rect.3) } else { (rect.0, rect.2) };
    let (room, quarter) = if vertical { (300.0, 75.0) } else { (400.0, 100.0) };
    assert_eq!(along(first[0]).1, quarter, "a quarter of the room");
    assert_eq!(along(first[2]), (room - quarter, quarter), "a quarter at the end");
    for (divider, pane) in [(3, 0), (4, 1)] {
      let (at, thick) = along(first[divider]);
      let (pane_at, pane_long) = along(first[pane]);
      assert!(
        ((at + thick / 2.0) - (pane_at + pane_long)).abs() < 1.0,
        "the divider after pane {pane} at its edge in the first frame: {} and {}",
        at + thick / 2.0,
        pane_at + pane_long
      );
    }
    for frame in 1..4 {
      stage.frame();
      assert_eq!(boxes(&mut stage, &ids), first, "frame {frame} is the first one");
    }
  }
}

#[test]
fn a_splitter_in_a_pane_that_was_put_back_settles_in_the_first_frame_too() {
  // The outer pane takes 300 of 600 where its share is 150: the inner splitter's room is 299, not the 449 it was
  // first measured at, and its last pane's least size of 200 takes from the one before it.
  let mut stage = Stage::first_frame(600.0, 300.0, |ctx| {
    let inner = Splitter::new()
      .id("inner")
      .pane(SplitterPane::new(content("b")).share(0.5).min(50.0))
      .pane(SplitterPane::new(content("c")).min(200.0))
      .key("inner")
      .mount(ctx);
    Splitter::new()
      .id("outer")
      .pane(SplitterPane::new(content("a")).share(0.25).min(300.0))
      .pane(SplitterPane::new(inner))
      .key("outer")
      .mount(ctx)
  });
  let ids = ["a", "b", "c", "outer-divider-0", "inner-divider-0"];
  let first = boxes(&mut stage, &ids);
  assert_eq!(first[0].2, 300.0);
  assert_eq!(first[1].2, 98.0);
  assert_eq!(first[2].2, 200.0);
  assert!(off_the_edge(&mut stage, "outer-divider-0", "a") < 1.0);
  assert!(off_the_edge(&mut stage, "inner-divider-0", "b") < 1.0);
  for frame in 1..4 {
    stage.frame();
    assert_eq!(boxes(&mut stage, &ids), first, "frame {frame} is the first one");
  }
}
