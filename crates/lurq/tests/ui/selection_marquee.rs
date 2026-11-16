//! A selection marquee: its rectangle against the board, what a drag over the surface tells the app from its start
//! to its end, the keys that add and toggle, Escape, a press that is no drag, a surface in a scroll container that
//! scrolls under the drag, what the rectangle touches, and a theme's other anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    events::{DragEvent, MouseButton, MouseButtonMask},
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row, ScrollVertical, Stack, Text},
  core::{ElementRef, Signal},
  layout::layout_kind::ScrollState,
  node::{Element, color::Color},
  ui::{
    Marquee, MarqueeMode, SelectionMarquee, SelectionMarqueePart, SelectionMarqueeRenderer, SelectionMarqueeView,
    Themed,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// What the app was told, in order.
type Told = Arc<Mutex<Vec<String>>>;

fn said(told: &Told) -> Vec<String> {
  std::mem::take(&mut *told.lock().unwrap())
}

/// Where the board's six tiles are on its surface of 230 by 120: 58 by 38, three across.
fn tile(index: usize) -> (f32, f32, f32, f32) {
  (
    14.0 + 72.0 * (index % 3) as f32,
    14.0 + 50.0 * (index / 3) as f32,
    58.0,
    38.0,
  )
}

/// The board's surface: six tiles laid out where the board has them, each pressed.
fn tiles(pressed: &Told) -> Stack {
  let mut surface = Stack::new().size(230.0, 120.0).id("tiles");
  for index in 0..6 {
    let (x, y, width, height) = tile(index);
    let pressed = pressed.clone();
    surface = surface.child(
      lurq::components::Button::empty()
        .id(format!("tile-{index}"))
        .absolute(x, y, width, height)
        .on_click(move |_| pressed.lock().unwrap().push(format!("press {index}"))),
    );
  }
  surface
}

fn words(what: &str, marquee: &Marquee) -> String {
  format!(
    "{what} {} {} {}x{} {}",
    marquee.x,
    marquee.y,
    marquee.width,
    marquee.height,
    marquee.mode.as_str()
  )
}

/// The board's surface in a marquee that says what it hears, 8 in from the corner of a window.
fn staged(mounted: bool, disabled: bool) -> (Stage, Told) {
  let told: Told = Arc::default();
  let heard = told.clone();
  let stage = Stage::new(move |ctx: &mut Ctx| {
    let (start, change, end, cancel) = (heard.clone(), heard.clone(), heard.clone(), heard.clone());
    let marquee = SelectionMarquee::new(tiles(&heard))
      .label("Tiles")
      .disabled(disabled)
      .id("surface")
      .on_start(move |marquee| start.lock().unwrap().push(words("start", marquee)))
      .on_change(move |marquee| change.lock().unwrap().push(words("change", marquee)))
      .on_change_end(move |marquee| end.lock().unwrap().push(words("end", marquee)))
      .on_cancel(move || cancel.lock().unwrap().push("cancel".to_owned()));
    let marquee: Element = if mounted { marquee.mount(ctx) } else { marquee.into() };
    Column::new().size(400.0, 300.0).padding(8.0).child(marquee).into()
  });
  (stage, told)
}

/// A press at `from` of the surface and moves to `to` in two steps, with the button still down.
fn drag_to(stage: &mut Stage, from: (f32, f32), to: (f32, f32)) {
  let at = |point: (f32, f32)| (8.0 + point.0, 8.0 + point.1);
  let (x, y) = at(from);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  let middle = at(((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0));
  stage.tree.mouse_move(middle.0, middle.1);
  stage.pass();
  let (x, y) = at(to);
  stage.tree.mouse_move(x, y);
  stage.pass();
}

fn release(stage: &mut Stage, at: (f32, f32)) {
  stage.tree.mouse_up(8.0 + at.0, 8.0 + at.1, MouseButton::Left);
  stage.pass();
}

#[test]
fn the_rectangle_is_the_boards() {
  let quiet: Told = Arc::default();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(
        SelectionMarquee::new(tiles(&quiet))
          .rect(76.0, 6.0, 146.0, 82.0)
          .id("surface"),
      )
      .into()
  });
  // The surface is as large as what it wraps, and the rectangle is where it was said.
  assert_eq!(stage.bounds("surface"), (8.0, 8.0, 230.0, 120.0));
  assert_eq!(stage.bounds("surface-marquee"), (84.0, 14.0, 146.0, 82.0));
  // On the muted info fill, in an edge of 1 in the info color, at 0.55.
  let fill = stage.fill("surface-marquee").expect("the rectangle's fill");
  let muted = light(PaletteColor::InfoMuted);
  match fill.content {
    lurq::layout::quad::QuadContent::Rect { color, .. } => {
      assert_eq!((color.r(), color.g(), color.b()), (muted.r(), muted.g(), muted.b()));
    }
    _ => unreachable!(),
  }
  let theme = lurq::app::theme::Theme::new();
  let marquee = <SelectionMarquee as Themed>::recipe()
    .resolve(&lurq::ui::Case::new(), &theme)
    .part(SelectionMarqueePart::Marquee)
    .rest();
  assert_eq!(marquee.opacity, Some(lurq::ui::Opacity::Value(0.55)));
  let edge = stage.border("surface-marquee").unwrap().border.unwrap().top.unwrap();
  let info = light(PaletteColor::Info);
  assert_eq!(edge.width, 1.0);
  assert_eq!(
    (edge.color.r(), edge.color.g(), edge.color.b()),
    (info.r(), info.g(), info.b())
  );
}

#[test]
fn a_drag_over_the_surface_tells_its_rectangle_from_its_start_to_its_end() {
  for mounted in [false, true] {
    let (mut stage, told) = staged(mounted, false);
    drag_to(&mut stage, (222.0, 6.0), (76.0, 88.0));
    // It is told once that it began, then each rectangle, from the corner the button went down at.
    assert_eq!(
      said(&told),
      [
        "start 149 6 73x41 replace",
        "change 149 6 73x41 replace",
        "change 76 6 146x82 replace"
      ],
      "mounted: {mounted}"
    );
    // Mounted, the rectangle is drawn; on its own it is only told.
    let drawn = stage
      .tree
      .get_element_by_id_mut("surface-marquee")
      .and_then(|element| element.bounds())
      .map(|rect| (rect.x, rect.y, rect.width, rect.height));
    assert_eq!(drawn, mounted.then_some((84.0, 14.0, 146.0, 82.0)));
    release(&mut stage, (76.0, 88.0));
    // The release over a tile ends the marquee and does not press the tile.
    assert_eq!(said(&told), ["end 76 6 146x82 replace"]);
    assert!(stage.tree.get_element_by_id_mut("surface-marquee").is_none());
  }
}

#[test]
fn a_press_that_does_not_move_is_no_marquee_and_presses_what_is_under_it() {
  let (mut stage, told) = staged(true, false);
  stage.click("tile-4");
  assert_eq!(said(&told), ["press 4"]);
  // A move of a pixel or two is a press still.
  let (x, y) = stage.center("tile-1");
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_move(x + 2.0, y + 1.0);
  stage.tree.mouse_up(x + 2.0, y + 1.0, MouseButton::Left);
  stage.pass();
  assert_eq!(said(&told), ["press 1"]);
}

#[test]
fn shift_adds_and_ctrl_puts_in_and_takes_out() {
  for (shift, ctrl, mode) in [(true, false, "add"), (false, true, "toggle"), (false, false, "replace")] {
    let (mut stage, told) = staged(true, false);
    stage.tree.mouse_move(18.0, 18.0);
    stage
      .tree
      .mouse_down_with_modifiers(18.0, 18.0, MouseButton::Left, shift, ctrl, false);
    stage.tree.mouse_move(58.0, 48.0);
    stage.pass();
    assert_eq!(
      said(&told),
      [
        format!("start 10 10 40x30 {mode}"),
        format!("change 10 10 40x30 {mode}")
      ]
    );
    release(&mut stage, (50.0, 40.0));
  }
}

#[test]
fn escape_ends_the_marquee_and_its_release_tells_nothing() {
  for mounted in [false, true] {
    let (mut stage, told) = staged(mounted, false);
    drag_to(&mut stage, (10.0, 10.0), (100.0, 100.0));
    said(&told);
    stage.key("Escape");
    assert_eq!(said(&told), ["cancel"], "mounted: {mounted}");
    assert!(stage.tree.get_element_by_id_mut("surface-marquee").is_none());
    release(&mut stage, (100.0, 100.0));
    assert!(said(&told).is_empty());
    // Escape with no marquee under way is not the surface's.
    stage.key("Escape");
    assert!(said(&told).is_empty());
  }
}

#[test]
fn the_rectangle_stays_on_the_surface() {
  let (mut stage, told) = staged(true, false);
  drag_to(&mut stage, (200.0, 100.0), (300.0, 200.0));
  assert_eq!(
    said(&told).last().map(String::as_str),
    Some("change 200 100 30x20 replace")
  );
  release(&mut stage, (300.0, 200.0));
  drag_to(&mut stage, (20.0, 20.0), (-50.0, -50.0));
  assert_eq!(said(&told).last().map(String::as_str), Some("change 0 0 20x20 replace"));
  release(&mut stage, (-50.0, -50.0));
}

#[test]
fn a_disabled_surface_takes_no_drag() {
  let (mut stage, told) = staged(true, true);
  drag_to(&mut stage, (10.0, 10.0), (100.0, 100.0));
  release(&mut stage, (100.0, 100.0));
  assert!(said(&told).is_empty());
}

#[test]
fn what_is_dragged_itself_on_the_surface_keeps_its_drag() {
  let told: Told = Arc::default();
  let heard = told.clone();
  let mut stage = Stage::new(move |ctx| {
    let (own, marquee) = (heard.clone(), heard.clone());
    let surface = Stack::new().size(230.0, 120.0).child(
      Row::new()
        .id("card")
        .absolute(20.0, 20.0, 60.0, 40.0)
        .start_drag_buttons(MouseButtonMask::LEFT)
        .on_drag_start(move |_: DragEvent| own.lock().unwrap().push("the card's drag".to_owned())),
    );
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(
        SelectionMarquee::new(surface)
          .on_start(move |_| marquee.lock().unwrap().push("marquee".to_owned()))
          .mount(ctx),
      )
      .into()
  });
  drag_to(&mut stage, (40.0, 40.0), (150.0, 100.0));
  release(&mut stage, (150.0, 100.0));
  assert_eq!(said(&told), ["the card's drag"]);
}

#[test]
fn a_surface_in_a_scroll_container_scrolls_under_the_drag_and_the_rectangle_keeps_its_start() {
  let state = ScrollState::new();
  let last: Arc<Mutex<Option<Marquee>>> = Arc::default();
  let (scroll, seen) = (state.clone(), last.clone());
  let mut stage = Stage::new(move |ctx| {
    let seen = seen.clone();
    let content = Column::new().size(200.0, 1000.0);
    Column::new()
      .size(400.0, 300.0)
      .child(
        ScrollVertical::new(
          SelectionMarquee::new(content)
            .scroll(&scroll)
            .on_change(move |marquee| *seen.lock().unwrap() = Some(*marquee))
            .mount(ctx),
        )
        .with_scroll_state(scroll.clone())
        .size(200.0, 200.0),
      )
      .into()
  });
  stage.tree.mouse_move(50.0, 50.0);
  stage.tree.mouse_down(50.0, 50.0, MouseButton::Left);
  // Held at the container's bottom edge: it scrolls, a step each frame, and the rectangle grows down the content.
  stage.tree.mouse_move(120.0, 198.0);
  for _ in 0..30 {
    stage.frame();
  }
  let scrolled = state.scroll_y();
  assert!(scrolled > 0.0, "the container scrolled");
  let marquee = last.lock().unwrap().expect("the rectangle");
  assert_eq!((marquee.x, marquee.y, marquee.width), (50.0, 50.0, 70.0));
  assert!(
    (marquee.height - (148.0 + scrolled)).abs() < 0.5,
    "{} high with {scrolled} scrolled",
    marquee.height
  );
  // It touches what is far down the content, and not what is past it.
  assert!(marquee.touches(60.0, 190.0 + scrolled, 20.0, 20.0));
  assert!(!marquee.touches(60.0, 199.0 + scrolled, 20.0, 20.0));
  stage.tree.mouse_up(120.0, 198.0, MouseButton::Left);
  stage.pass();
}

#[test]
fn a_rectangle_says_what_it_touches_and_what_is_selected_with_it() {
  // Told to the app in the middle of a drag: the board's rectangle, which shares room with its tiles 2, 3, 5 and 6.
  let told: Arc<Mutex<Option<Marquee>>> = Arc::default();
  let (heard, quiet) = (told.clone(), Told::default());
  let tile_ref = ElementRef::new();
  let touched = Signal::new(false);
  let (at, touches) = (tile_ref.clone(), touched.clone());
  let mut stage = Stage::new(move |ctx| {
    let (heard, at, touches) = (heard.clone(), at.clone(), touches.clone());
    let surface = tiles(&quiet).child(
      Row::new()
        .ref_element(tile_ref.clone())
        .absolute(158.0, 14.0, 58.0, 38.0),
    );
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(
        SelectionMarquee::new(surface)
          .on_change(move |marquee| {
            *heard.lock().unwrap() = Some(*marquee);
            touches.set(marquee.touches_element(&at));
          })
          .mount(ctx),
      )
      .into()
  });
  stage.tree.mouse_move(8.0 + 222.0, 8.0 + 6.0);
  stage
    .tree
    .mouse_down_with_modifiers(8.0 + 222.0, 8.0 + 6.0, MouseButton::Left, false, true, false);
  stage.tree.mouse_move(8.0 + 76.0, 8.0 + 88.0);
  stage.pass();
  let marquee = told.lock().unwrap().expect("the rectangle");
  let over: Vec<usize> = (0..6)
    .filter(|index| {
      let (x, y, width, height) = tile(*index);
      marquee.touches(x, y, width, height)
    })
    .collect();
  assert_eq!(over, [1, 2, 4, 5]);
  // An element of the surface is touched by where it is laid out in the window.
  assert!(touched.get());
  // Only the tiles wholly in it are contained.
  let (x, y, width, height) = tile(4);
  assert!(!marquee.contains(x, y, width, height));
  let (x, y, width, height) = tile(1);
  assert!(marquee.contains(x, y, width, height));
  // With Ctrl, what it touches goes in, and out where it was in.
  assert_eq!(marquee.mode, MarqueeMode::Toggle);
  assert_eq!(marquee.select(&[0, 1], over.clone()), [0, 2, 4, 5]);
  let mut add = marquee;
  add.mode = MarqueeMode::Add;
  assert_eq!(add.select(&[0, 1], over.clone()), [0, 1, 2, 4, 5]);
  let mut replace = marquee;
  replace.mode = MarqueeMode::Replace;
  assert_eq!(replace.select(&[0, 1], over), [1, 2, 4, 5]);
  stage.tree.mouse_up(8.0 + 76.0, 8.0 + 88.0, MouseButton::Left);
  stage.pass();
}

#[test]
fn a_theme_gives_the_marquee_another_anatomy_and_another_rule() {
  let app = App::new();
  let renderer: SelectionMarqueeRenderer = Arc::new(|view: SelectionMarqueeView| {
    Row::new()
      .child(Text::new(if view.rect.is_some() { "dragged" } else { "at rest" }))
      .into()
  });
  app.theme().set_renderer::<SelectionMarquee>(renderer);
  let stage = Stage::in_app(app, |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(SelectionMarquee::new(Row::new().size(100.0, 100.0)).rect(0.0, 0.0, 10.0, 10.0))
      .into()
  });
  assert!(stage.paints("dragged"));

  let app = App::new();
  app.theme().patch_recipe::<SelectionMarquee>(|recipe| {
    recipe.base(SelectionMarqueePart::Marquee, |s| {
      s.background(PaletteColor::Primary).opacity(1.0)
    })
  });
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        SelectionMarquee::new(Row::new().size(100.0, 100.0))
          .rect(10.0, 10.0, 40.0, 40.0)
          .id("surface"),
      )
      .into()
  });
  assert_eq!(stage.fill_color("surface-marquee"), Some(light(PaletteColor::Primary)));
  assert_eq!(<SelectionMarquee as Themed>::NAME, "selection_marquee");
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_the_surface_by_its_label() {
  let (mut stage, _) = staged(false, false);
  let name = stage
    .tree
    .get_element_by_id_mut("surface")
    .and_then(|element| element.description("a11y_name"));
  assert_eq!(name.as_deref(), Some("Tiles"));
}
