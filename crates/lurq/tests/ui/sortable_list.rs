//! A sortable list: its rows against the board, a drag of a row and of its handle, what follows the pointer and
//! what shows where it would go, Escape, the keys that move a row, a selection dragged together, a disabled row,
//! what the app is told, a title longer than its room, the density, tooling, and a theme's other anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    events::MouseButton,
    theme::{Density, PaletteColor, ThemePalette},
  },
  components::{Column, Row, Text},
  core::Signal,
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{
    Interaction, SortableItem, SortableList, SortableListPart, SortableListRenderer, SortableListView, Themed,
    move_rows,
  },
};

use super::stage::Stage;

type Id = &'static str;

const GRIP: &str = "\u{E038}";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// What the app was told, in order.
type Told = Arc<Mutex<Vec<String>>>;

fn said(told: &Told) -> Vec<String> {
  std::mem::take(&mut *told.lock().unwrap())
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn close_to(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

/// The board's rows, each with its name as its value and its id.
const BOARD: [Id; 4] = ["Buttons", "Inputs", "Tabs", "Toggles"];

fn rows(order: &[Id]) -> Vec<SortableItem<Id>> {
  order
    .iter()
    .map(|name| SortableItem::new(*name, name).id(*name))
    .collect()
}

/// What a test keeps of a list: its rows in the app's order, what is selected, and what the app was told.
#[derive(Clone)]
struct Held {
  order: Signal<Vec<Id>>,
  selected: Signal<Vec<Id>>,
  told: Told,
}

impl Held {
  fn new() -> Self {
    Self {
      order: Signal::new(BOARD.to_vec()),
      selected: Signal::new(Vec::new()),
      told: Arc::default(),
    }
  }

  /// The list of the app's rows, which reorders them as it is told and says what it heard.
  fn list(&self, selects: bool) -> SortableList<Id> {
    let (one, many) = (self.order.clone(), self.order.clone());
    let (told_one, told_many, told_selected) = (self.told.clone(), self.told.clone(), self.told.clone());
    let mut list = SortableList::new(rows(&self.order.get()))
      .label("Sections")
      .id("list")
      .style(SortableListPart::Root, |s| s.width(300.0))
      .on_reorder(move |from, to| {
        told_one.lock().unwrap().push(format!("reorder {from} {to}"));
        one.update(|rows| {
          let row = rows.remove(from);
          rows.insert(to, row);
        });
      })
      .on_reorder_many(move |rows, to| {
        told_many.lock().unwrap().push(format!("reorder many {rows:?} {to}"));
        many.update(|order| move_rows(order, rows, to));
      });
    if selects {
      list = list
        .selected(self.selected.clone())
        .on_selected_change(move |selected| told_selected.lock().unwrap().push(format!("selected {selected:?}")));
    }
    list
  }
}

/// A stage of the held list, 8 in from the corner of a window, mounted or not, with a selection or without.
fn staged(mounted: bool, selects: bool) -> (Stage, Held) {
  let held = Held::new();
  let scene = held.clone();
  let stage = Stage::new(move |ctx: &mut Ctx| {
    let list = scene.list(selects);
    let list: Element = if mounted { list.mount(ctx) } else { list.into() };
    Column::new().size(400.0, 300.0).padding(8.0).child(list).into()
  });
  (stage, held)
}

/// A press at the middle of `id`, a move of `down` in small steps, and the release.
fn drag(stage: &mut Stage, id: &str, down: f32) {
  let (x, y) = stage.center(id);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  for step in 1..=10 {
    stage.tree.mouse_move(x + 3.0, y + down * step as f32 / 10.0);
    stage.pass();
  }
  stage.tree.mouse_up(x + 3.0, y + down, MouseButton::Left);
  stage.pass();
}

/// A key with Alt, Shift or Ctrl held.
fn key_with(stage: &mut Stage, key: &str, shift: bool, ctrl: bool, alt: bool) {
  stage.tree.key_down(key.to_owned(), key.to_owned(), shift, ctrl, alt);
  stage.tree.key_up(key.to_owned(), key.to_owned(), shift, ctrl, alt);
  stage.pass();
}

#[test]
fn the_list_is_the_boards_rows() {
  let (mut stage, _) = staged(false, false);
  // Rows of 36, 4 apart, as wide as the list.
  for (index, name) in BOARD.iter().enumerate() {
    assert_eq!(
      stage.bounds(name),
      (8.0, 8.0 + 40.0 * index as f32, 300.0, 36.0),
      "{name}"
    );
  }
  // On the card's fill, in an edge of 1 in the border's color.
  assert_eq!(stage.fill_color("Buttons"), Some(light(PaletteColor::Card)));
  let edge = stage
    .border("Buttons")
    .expect("the row's edge")
    .border
    .unwrap()
    .top
    .unwrap();
  assert_eq!((edge.width, edge.color), (1.0, light(PaletteColor::Border)));
  // The grip of 14 is 6 in, in the muted foreground, and the text 8 after it at 13.
  let grip = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == GRIP))
    .expect("the grip");
  assert_eq!((grip.x, grip.width), (14.0, 14.0));
  assert_eq!(stage.text(GRIP).0, light(PaletteColor::MutedForeground));
  assert_eq!(stage.text_quad("Buttons").unwrap().x, 8.0 + 6.0 + 14.0 + 8.0);
  assert_eq!(stage.text("Buttons"), (light(PaletteColor::CardForeground), 13.0));
}

#[test]
fn a_drag_tells_where_the_row_was_let_go_and_a_small_move_tells_nothing() {
  for mounted in [false, true] {
    let (mut stage, held) = staged(mounted, false);
    // Tabs past the middle of Inputs: it goes before it.
    drag(&mut stage, "Tabs", -45.0);
    assert_eq!(said(&held.told), ["reorder 2 1"], "mounted: {mounted}");
    assert_eq!(held.order.get(), ["Buttons", "Tabs", "Inputs", "Toggles"]);
    assert_eq!(stage.bounds("Tabs").1, 48.0);
    // To the end of the list.
    drag(&mut stage, "Buttons", 140.0);
    assert_eq!(said(&held.told), ["reorder 0 3"]);
    assert_eq!(held.order.get(), ["Tabs", "Inputs", "Toggles", "Buttons"]);
    // Less than half a row is where it was, and a pixel or two is a press: neither tells anything.
    drag(&mut stage, "Inputs", 15.0);
    drag(&mut stage, "Inputs", 2.0);
    assert!(said(&held.told).is_empty());
  }
}

#[test]
fn the_dragged_row_follows_the_pointer_and_the_rows_make_room_for_the_line_as_the_board_has_them() {
  let (mut stage, held) = staged(true, false);
  let (x, y) = stage.center("Tabs");
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  for step in 1..=10 {
    stage.tree.mouse_move(x + 2.0 * step as f32, y - 6.0 * step as f32);
    stage.pass();
  }
  // The board's list while Tabs is dragged to between Buttons and Inputs: the line stands in the flow, 4 from
  // each, so those two are 10 apart, and the list is 162 high: four rows, three rooms of 4 and one of 10.
  assert_eq!(stage.bounds("list"), (8.0, 8.0, 300.0, 162.0));
  assert_eq!(stage.bounds("Buttons"), (8.0, 8.0, 300.0, 36.0));
  assert_eq!(stage.bounds("Inputs"), (8.0, 54.0, 300.0, 36.0));
  assert_eq!(stage.bounds("Toggles"), (8.0, 134.0, 300.0, 36.0));
  // The ghost stands where Tabs is in the list, as high, on the muted fill in the edge of an input.
  let ghost = stage
    .quads()
    .into_iter()
    .find(|quad| {
      (quad.x, quad.y, quad.width, quad.height) == (8.0, 94.0, 300.0, 36.0)
        && quad
          .border
          .as_ref()
          .and_then(|border| border.top.as_ref())
          .is_some_and(|side| side.color.r() == light(PaletteColor::Input).r())
    })
    .is_some();
  assert!(ghost, "the ghost in the row's place");
  // The row itself is drawn over the list where the pointer has it, 40 narrower than the list as the board's is
  // (260 over 300), taken off about the pointer: it went down in the row's middle, so 20 at each side.
  let title = stage.text_quad("Tabs").expect("the dragged row");
  assert!(close_to(title.x, 8.0 + 20.0 + 20.0 + 28.0), "{}", title.x);
  assert!(title.y < 88.0 - 40.0, "{}", title.y);
  let card = light(PaletteColor::Card);
  let lifted = stage
    .quads()
    .into_iter()
    .find(|quad| quad.width == 260.0 && matches!(&quad.content, QuadContent::Rect { color, .. } if *color == card))
    .expect("the row over the list");
  // Turned 2 degrees about its middle, its corner is within 5 of where the row unturned has it: 48, 28.
  assert_eq!(lifted.height, 36.0);
  assert!(
    (lifted.x - 48.0).abs() < 1.0 && (lifted.y - 28.0).abs() < 5.0,
    "at {}, {}",
    lifted.x,
    lifted.y
  );
  // The line of 2 in the info color, between Buttons and Inputs, with its dot of 6 at its start.
  let info = light(PaletteColor::Info);
  let line = stage
    .quads()
    .into_iter()
    .find(|quad| quad.height == 2.0 && matches!(&quad.content, QuadContent::Rect { color, .. } if *color == info))
    .expect("the line");
  assert_eq!(line.y, 48.0, "4 under Buttons, and 4 over Inputs");
  assert_eq!((line.x, line.x + line.width), (14.0, 308.0));
  let dot = stage
    .quads()
    .into_iter()
    .find(|quad| quad.height == 6.0 && matches!(&quad.content, QuadContent::Rect { color, .. } if *color == info))
    .expect("the dot");
  assert_eq!((dot.x, dot.width), (8.0, 6.0));
  assert_eq!(dot.y + 3.0, 49.0, "on the middle of the line");
  // One row dragged has no count.
  assert!(!stage.paints("1"));
  stage.tree.mouse_up(x + 20.0, y - 60.0, MouseButton::Left);
  stage.pass();
  assert_eq!(said(&held.told), ["reorder 2 1"]);
  // Nothing is left over the list, which is as high as its rows again.
  assert_eq!(stage.bounds("Tabs"), (8.0, 48.0, 300.0, 36.0));
  assert_eq!(stage.bounds("list"), (8.0, 8.0, 300.0, 156.0));
  assert!(
    !stage
      .quads()
      .into_iter()
      .any(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == info))
  );
}

/// A press in the middle of Tabs and one move up past Inputs: the line shows between Buttons and Inputs.
fn lift_tabs(stage: &mut Stage) -> (f32, f32) {
  let (x, y) = stage.center("Tabs");
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_move(x, y - 60.0);
  (x, y - 60.0)
}

#[test]
fn the_rows_move_to_make_room_for_the_line_and_do_not_jump() {
  let (mut stage, _) = staged(true, false);
  stage.pass();
  let (x, y) = lift_tabs(&mut stage);
  // A few frames after the line showed, Inputs is on its way from 48 to 54, and further a few frames on.
  for _ in 0..3 {
    stage.frame();
  }
  let early = stage.bounds("Inputs").1;
  for _ in 0..3 {
    stage.frame();
  }
  let later = stage.bounds("Inputs").1;
  assert!(early > 48.0 && early < 54.0, "{early}");
  assert!(later > early && later < 54.0, "{later} after {early}");
  stage.pass();
  assert_eq!(stage.bounds("Inputs").1, 54.0);
  // Back where it came from, letting go would move nothing: no line, and the rows close up again, in time too.
  stage.tree.mouse_move(x, y + 60.0 + 6.0);
  for _ in 0..4 {
    stage.frame();
  }
  let closing = stage.bounds("Inputs").1;
  assert!(closing > 48.0 && closing < 54.0, "{closing}");
  stage.pass();
  assert_eq!(stage.bounds("Inputs").1, 48.0);
  assert_eq!(stage.bounds("list").3, 156.0);
  // Before the first row the line has the room of one side: the list is 6 higher, and every row 6 lower.
  stage.tree.mouse_move(x, y - 60.0);
  stage.pass();
  assert_eq!(stage.bounds("Buttons").1, 14.0);
  assert_eq!(stage.bounds("list").3, 162.0);
  stage.tree.mouse_up(x, y - 60.0, MouseButton::Left);
  stage.pass();
}

#[test]
fn where_motion_is_reduced_the_rows_make_room_at_once() {
  let held = Held::new();
  let scene = held.clone();
  let app = App::new();
  app.theme().set_reduced_motion(Some(true));
  let mut stage = Stage::in_app(app, move |ctx: &mut Ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(scene.list(false).mount(ctx))
      .into()
  });
  stage.pass();
  let (x, y) = lift_tabs(&mut stage);
  stage.frame();
  stage.frame();
  assert_eq!(stage.bounds("Inputs").1, 54.0);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

#[test]
fn escape_puts_a_dragged_row_back_and_its_release_tells_nothing() {
  for mounted in [false, true] {
    let (mut stage, held) = staged(mounted, false);
    let (x, y) = stage.center("Tabs");
    stage.tree.mouse_move(x, y);
    stage.tree.mouse_down(x, y, MouseButton::Left);
    for step in 1..=5 {
      stage.tree.mouse_move(x, y - 12.0 * step as f32);
      stage.pass();
    }
    stage.key("Escape");
    stage.tree.mouse_up(x, y - 60.0, MouseButton::Left);
    stage.pass();
    assert!(said(&held.told).is_empty(), "mounted: {mounted}");
    assert_eq!(stage.bounds("Tabs"), (8.0, 88.0, 300.0, 36.0));
  }
}

#[test]
fn alt_with_a_key_moves_the_focused_row_and_the_focus_goes_with_it() {
  for mounted in [false, true] {
    let (mut stage, held) = staged(mounted, false);
    // One stop of Tab; Up and Down move the focus and no row.
    stage.key("Tab");
    assert_eq!(focused(&stage).as_deref(), Some("Buttons"), "mounted: {mounted}");
    stage.key("ArrowDown");
    assert_eq!(focused(&stage).as_deref(), Some("Inputs"));
    assert!(said(&held.told).is_empty());
    key_with(&mut stage, "ArrowDown", false, false, true);
    assert_eq!(said(&held.told), ["reorder 1 2"]);
    assert_eq!(held.order.get(), ["Buttons", "Tabs", "Inputs", "Toggles"]);
    assert_eq!(focused(&stage).as_deref(), Some("Inputs"));
    key_with(&mut stage, "Home", false, false, true);
    assert_eq!(said(&held.told), ["reorder 2 0"]);
    assert_eq!(focused(&stage).as_deref(), Some("Inputs"));
    // Past the first there is nowhere to go.
    key_with(&mut stage, "ArrowUp", false, false, true);
    assert!(said(&held.told).is_empty());
    key_with(&mut stage, "End", false, false, true);
    assert_eq!(said(&held.told), ["reorder 0 3"]);
    assert_eq!(held.order.get(), ["Buttons", "Tabs", "Toggles", "Inputs"]);
    assert_eq!(focused(&stage).as_deref(), Some("Inputs"));
    key_with(&mut stage, "ArrowUp", false, false, true);
    assert_eq!(said(&held.told), ["reorder 3 2"]);
  }
}

#[test]
fn a_selection_is_pressed_as_a_list_of_files_and_dragged_together() {
  let (mut stage, held) = staged(true, true);
  stage.click("Buttons");
  assert_eq!(held.selected.get(), ["Buttons"]);
  assert_eq!(said(&held.told), [r#"selected ["Buttons"]"#]);
  // With Ctrl one more; the selected rows are in the highlight.
  let (x, y) = stage.center("Tabs");
  stage.tree.mouse_move(x, y);
  stage
    .tree
    .mouse_down_with_modifiers(x, y, MouseButton::Left, false, true, false);
  stage
    .tree
    .mouse_up_with_modifiers(x, y, MouseButton::Left, false, true, false);
  stage.pass();
  assert_eq!(held.selected.get(), ["Buttons", "Tabs"]);
  assert_eq!(stage.fill_color("Tabs"), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.fill_color("Inputs"), Some(light(PaletteColor::Card)));
  said(&held.told);

  // A drag of one of them drags both: the row under the pointer counts them, and each leaves a ghost.
  let (x, y) = stage.center("Tabs");
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  for step in 1..=10 {
    stage.tree.mouse_move(x, y + 5.0 * step as f32);
    stage.pass();
  }
  assert_eq!(stage.text("2"), (Color::from_hex("#FFFFFF"), 11.0));
  let count = stage.text_quad("2").unwrap();
  // The row over the list is 260 wide from 28: the badge of 20 is 4 past its end.
  assert!(
    count.x > 8.0 + 20.0 + 260.0 - 20.0 && count.x < 8.0 + 20.0 + 260.0 + 4.0,
    "on the corner at the row's end: {}",
    count.x
  );
  assert!(
    !stage.paints("Buttons"),
    "a ghost stands in the place of the other row dragged"
  );
  stage.tree.mouse_up(x, y + 50.0, MouseButton::Left);
  stage.pass();
  // The two go after Toggles together, in their order; the release selects nothing else.
  assert_eq!(said(&held.told), ["reorder many [0, 2] 2"]);
  assert_eq!(held.order.get(), ["Inputs", "Toggles", "Buttons", "Tabs"]);
  assert_eq!(held.selected.get(), ["Buttons", "Tabs"]);

  // Alt with a key moves them together too, and Ctrl+A selects every row.
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("Buttons"));
  key_with(&mut stage, "ArrowUp", false, false, true);
  assert_eq!(said(&held.told), ["reorder many [2, 3] 1"]);
  assert_eq!(held.order.get(), ["Inputs", "Buttons", "Tabs", "Toggles"]);
  assert_eq!(focused(&stage).as_deref(), Some("Buttons"));
  key_with(&mut stage, "a", false, true, false);
  assert_eq!(held.selected.get().len(), 4);
  // Space takes the focused row out.
  stage.key(" ");
  assert_eq!(held.selected.get().len(), 3);
  assert!(!held.selected.get().contains(&"Buttons"));
}

#[test]
fn a_list_without_a_selection_selects_nothing() {
  let (mut stage, held) = staged(true, false);
  stage.click("Buttons");
  assert!(held.selected.get().is_empty() && said(&held.told).is_empty());
  assert_eq!(stage.fill_color("Buttons"), Some(light(PaletteColor::Card)));
  assert_eq!(focused(&stage).as_deref(), Some("Buttons"));
}

#[test]
fn a_row_is_dragged_by_its_handle_alone_where_the_list_says_so() {
  let told: Told = Arc::default();
  let heard = told.clone();
  let mut stage = Stage::new(move |ctx| {
    let heard = heard.clone();
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(
        SortableList::new(rows(&BOARD))
          .handle_only(true)
          .on_reorder(move |from, to| heard.lock().unwrap().push(format!("reorder {from} {to}")))
          .mount(ctx),
      )
      .into()
  });
  drag(&mut stage, "Tabs", -60.0);
  assert!(said(&told).is_empty(), "the row itself is not what drags it");
  drag(&mut stage, "Tabs-handle", -60.0);
  assert_eq!(said(&told), ["reorder 2 1"]);

  // Without a grip nothing is drawn before the text.
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(SortableList::new(rows(&BOARD)).handle(false))
      .into()
  });
  assert!(
    !stage
      .quads()
      .into_iter()
      .any(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == GRIP))
  );
  assert_eq!(stage.text_quad("Buttons").unwrap().x, 8.0 + 6.0);
  let _ = stage.bounds("Buttons");
}

#[test]
fn a_disabled_row_stays_where_it_is_and_a_disabled_list_moves_nothing() {
  let told: Told = Arc::default();
  let heard = told.clone();
  let mut stage = Stage::new(move |ctx| {
    let heard = heard.clone();
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(
        SortableList::new([
          SortableItem::new("a", "a").id("a"),
          SortableItem::new("b", "b").id("b").disabled(true),
          SortableItem::new("c", "c").id("c"),
        ])
        .on_reorder(move |from, to| heard.lock().unwrap().push(format!("reorder {from} {to}")))
        .mount(ctx),
      )
      .into()
  });
  drag(&mut stage, "b", 60.0);
  assert!(said(&told).is_empty());
  // The keys pass it by, and it is not moved by them; another row is still put after it.
  stage.key("Tab");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("c"));
  drag(&mut stage, "a", 50.0);
  assert_eq!(said(&told), ["reorder 0 1"]);

  let heard = told.clone();
  let mut stage = Stage::new(move |ctx| {
    let heard = heard.clone();
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(
        SortableList::new(rows(&BOARD))
          .disabled(true)
          .on_reorder(move |from, to| heard.lock().unwrap().push(format!("reorder {from} {to}")))
          .mount(ctx),
      )
      .into()
  });
  drag(&mut stage, "Tabs", -60.0);
  stage.key("Tab");
  key_with(&mut stage, "ArrowDown", false, false, true);
  assert!(said(&told).is_empty());
}

#[test]
fn a_row_has_a_description_an_icon_and_what_stands_at_its_end_and_a_long_title_is_cut() {
  let long = "A section with a name that is far longer than a row of the list has room for";
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(
        SortableList::new([
          SortableItem::new(0, long).id("long").trailing(Text::new("12")),
          SortableItem::new(1, "Inputs")
            .id("inputs")
            .description("Text, number, password")
            .icon(lurq::ui::icons::FILE),
        ])
        .style(SortableListPart::Root, |s| s.width(300.0)),
      )
      .into()
  });
  let (x, _, width, height) = stage.bounds("long");
  assert_eq!((width, height), (300.0, 36.0));
  // What stands at the end keeps its room, 10 from the row's end, and the title is cut before it.
  let end = stage.text_quad("12").expect("what stands at the row's end");
  assert!(close_to(end.x + end.width, x + width - 10.0), "{}", end.x + end.width);
  let title = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text.starts_with("A section")))
    .expect("the title");
  let shown = if title.clip.active {
    (title.x + title.width).min(title.clip.x + title.clip.width)
  } else {
    title.x + title.width
  };
  assert!(
    shown <= end.x,
    "the title ends at {shown}, what is after it starts at {}",
    end.x
  );
  // A row of two lines is as high as a list item of two, and its icon stands after its grip.
  assert_eq!(stage.bounds("inputs").3, 52.0);
  assert!(stage.paints("Text, number, password"));
  assert_eq!(
    stage.text_quad("Inputs").unwrap().x,
    8.0 + 6.0 + 14.0 + 8.0 + 16.0 + 8.0
  );
}

#[test]
fn a_rows_height_follows_the_density() {
  for (density, height) in [
    (Density::Compact, 28.0),
    (Density::Default, 36.0),
    (Density::Comfortable, 44.0),
    (Density::Touch, 56.0),
  ] {
    let app = App::new();
    app.theme().set_density(density);
    let mut stage = Stage::in_app(app, move |_| {
      Column::new()
        .size(400.0, 300.0)
        .child(SortableList::new(rows(&BOARD)))
        .into()
    });
    assert_eq!(stage.bounds("Buttons").3, height, "{density:?}");
    assert_eq!(stage.bounds("Inputs").1, height + 4.0, "{density:?}: 4 apart");
  }
}

#[test]
fn the_list_is_as_wide_as_its_room() {
  let mut stage = Stage::new(|ctx| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Column::new()
          .width(200.0)
          .child(SortableList::new(rows(&BOARD)).id("list").mount(ctx)),
      )
      .into()
  });
  assert_eq!(stage.bounds("list").2, 200.0);
  assert_eq!(stage.bounds("Buttons").2, 200.0);
}

#[test]
fn the_first_frame_of_a_mounted_list_is_the_settled_one() {
  let mut stage = Stage::first_frame(400.0, 300.0, |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .child(SortableList::new(rows(&BOARD)).mount(ctx))
      .into()
  });
  let first = stage.picture();
  stage.frame();
  assert_eq!(first, stage.picture());
}

#[test]
fn a_forced_focus_draws_the_ring_on_the_row_tab_would_stop_on() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(SortableList::new(rows(&BOARD)).force(Some(Interaction::FocusVisible)))
      .into()
  });
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  let _ = stage.bounds("Buttons");
}

#[test]
fn rows_are_moved_together_as_the_app_is_asked() {
  let mut list = vec!["a", "b", "c", "d", "e"];
  move_rows(&mut list, &[0, 1], 3);
  assert_eq!(list, ["c", "d", "e", "a", "b"]);
  move_rows(&mut list, &[4, 1], 0);
  assert_eq!(list, ["d", "b", "c", "e", "a"]);
  // A place past the end is the end, and a row that is not there is no row.
  move_rows(&mut list, &[0, 9], 9);
  assert_eq!(list, ["b", "c", "e", "a", "d"]);
}

#[test]
fn a_theme_gives_the_list_another_anatomy_and_another_rule() {
  let app = App::new();
  let renderer: SortableListRenderer =
    Arc::new(|view: SortableListView| Row::new().child(Text::new(&format!("{} rows", view.rows.len()))).into());
  app.theme().set_renderer::<SortableList<Id>>(renderer);
  let stage = Stage::in_app(app, |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(SortableList::new(rows(&BOARD)))
      .into()
  });
  assert!(stage.paints("4 rows"));

  let app = App::new();
  app
    .theme()
    .patch_recipe::<SortableList<Id>>(|recipe| recipe.base(SortableListPart::Root, |s| s.gap(10.0)));
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(SortableList::new(rows(&BOARD)))
      .into()
  });
  assert_eq!(stage.bounds("Inputs").1, 46.0);
  assert_eq!(<SortableList as Themed>::NAME, "sortable_list");
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_list_of_items_and_a_listbox_where_rows_are_selected() {
  let read = |stage: &mut Stage, id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  let (mut stage, _) = staged(false, false);
  assert_eq!(read(&mut stage, "list", "a11y_role").as_deref(), Some("list"));
  assert_eq!(read(&mut stage, "list", "a11y_name").as_deref(), Some("Sections"));
  assert_eq!(read(&mut stage, "Tabs", "a11y_role").as_deref(), Some("listitem"));
  assert_eq!(read(&mut stage, "Tabs", "a11y_name").as_deref(), Some("Tabs"));
  assert_eq!(read(&mut stage, "Tabs", "a11y_posinset").as_deref(), Some("3"));
  assert_eq!(read(&mut stage, "Tabs", "a11y_setsize").as_deref(), Some("4"));
  assert_eq!(read(&mut stage, "Tabs", "a11y_selected"), None);

  let (mut stage, held) = staged(false, true);
  held.selected.set(vec!["Tabs"]);
  stage.pass();
  assert_eq!(read(&mut stage, "list", "a11y_role").as_deref(), Some("listbox"));
  assert_eq!(
    read(&mut stage, "list", "a11y_multiselectable").as_deref(),
    Some("true")
  );
  assert_eq!(read(&mut stage, "Tabs", "a11y_role").as_deref(), Some("option"));
  assert_eq!(read(&mut stage, "Tabs", "a11y_selected").as_deref(), Some("true"));
  assert_eq!(read(&mut stage, "Inputs", "a11y_selected").as_deref(), Some("false"));
}
