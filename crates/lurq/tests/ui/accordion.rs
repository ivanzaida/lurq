//! A collapsible and an accordion: their headers and lines against the board, what a press or a key opens and
//! closes, a disabled one, the height an item opens and closes with, and a theme's other anatomy.

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Button, Column, Row},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{
    Accordion, AccordionItem, AccordionPart, Collapsible, CollapsiblePart, CollapsibleRenderer, CollapsibleView,
    Interaction,
  },
};

use super::stage::Stage;

const CHEVRON_DOWN: &str = "\u{E019}";
const CHEVRON_UP: &str = "\u{E01C}";
const CHEVRONS_VERTICAL: &str = "\u{E01D}";

const GPU: &str = "Is lurq GPU-accelerated?";
const DARK: &str = "Does it support dark mode?";
const THEME: &str = "Can I bring my own theme?";
const GPU_ANSWER: &str = "Yes, through wgpu.";
const DARK_ANSWER: &str = "Every theme has two appearances.";
const THEME_ANSWER: &str = "Yes: a theme is a value.";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn items() -> [AccordionItem<u8>; 3] {
  [
    AccordionItem::text(0, GPU, GPU_ANSWER),
    AccordionItem::text(1, DARK, DARK_ANSWER),
    AccordionItem::text(2, THEME, THEME_ANSWER),
  ]
}

/// The questions of the board, one open at a time, 360 wide.
fn faq(open: &Signal<Option<u8>>) -> Stage {
  let open = open.clone();
  Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(
        Accordion::new(open.clone(), items())
          .style(AccordionPart::Root, |s| s.width(360.0))
          .id("faq"),
      )
      .into()
  })
}

/// A press of the pointer on the text `content`: a header is pressed by its label.
fn press(stage: &mut Stage, content: &str) {
  let quad = stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("{content:?} should be painted"));
  let (x, y) = (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

fn top(stage: &Stage, content: &str) -> f32 {
  stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("{content:?} should be painted"))
    .y
}

fn count(stage: &Stage, content: &str) -> usize {
  stage
    .quads()
    .iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content))
    .count()
}

/// The lines under the items: a border along the bottom alone, in the border's color.
fn lines(stage: &Stage) -> Vec<Quad> {
  let mut lines: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      quad.border.is_some_and(|border| {
        border.top.is_none()
          && border
            .bottom
            .is_some_and(|line| line.color == light(PaletteColor::Border))
      })
    })
    .collect();
  lines.sort_by(|a, b| a.y.total_cmp(&b.y));
  lines
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

#[test]
fn an_accordion_is_its_headers_with_a_line_under_each_as_the_board_draws_it() {
  let open = Signal::new(Some(0));
  let mut stage = faq(&open);
  let (x, y, width, _) = stage.bounds("faq");
  assert_eq!(width, 360.0);

  // A line of 1 in the border's color under each item, as wide as the accordion.
  let lines = lines(&stage);
  assert_eq!(lines.len(), 3, "a line under each item");
  for line in &lines {
    assert_eq!((line.x, line.width), (x, 360.0));
    assert_eq!(line.border.unwrap().bottom.unwrap().width, 1.0);
  }

  // A header: its label 14 under the top of its item, in the body's type at medium weight and the foreground.
  assert!(close(top(&stage, GPU) - y, 14.0), "{}", top(&stage, GPU) - y);
  assert!(close(top(&stage, DARK) - (lines[0].y + lines[0].height), 14.0));
  assert_eq!(stage.text(GPU), (light(PaletteColor::Foreground), 14.0));
  // At its end a chevron of 16 in the muted foreground: up on the open item, down on the closed ones.
  assert_eq!(count(&stage, CHEVRON_UP), 1);
  assert_eq!(count(&stage, CHEVRON_DOWN), 2);
  assert_eq!(stage.text(CHEVRON_UP), (light(PaletteColor::MutedForeground), 16.0));
  let chevron = stage.text_quad(CHEVRON_UP).unwrap();
  assert!(close(chevron.x + chevron.width, x + 360.0), "at the end of the header");
  // 14 under the label of a closed item is the line.
  let label = stage.text_quad(DARK).unwrap();
  assert!(close(lines[1].y + lines[1].height - (label.y + label.height), 14.0));

  // What the open item holds: in the body's type, in the muted foreground, 14 over the line.
  assert_eq!(stage.text(GPU_ANSWER), (light(PaletteColor::MutedForeground), 14.0));
  let answer = stage.text_quad(GPU_ANSWER).unwrap();
  assert!(close(lines[0].y + lines[0].height - (answer.y + answer.height), 14.0));
  assert!(!stage.paints(DARK_ANSWER), "a closed item shows nothing");
  assert!(!stage.paints(THEME_ANSWER));
}

#[test]
fn one_item_is_open_at_a_time_and_a_press_of_the_open_one_closes_it() {
  let open = Signal::new(None);
  let mut stage = faq(&open);
  assert!(!stage.paints(GPU_ANSWER) && !stage.paints(DARK_ANSWER));

  press(&mut stage, GPU);
  assert_eq!(open.get(), Some(0));
  assert!(stage.paints(GPU_ANSWER));

  press(&mut stage, DARK);
  assert_eq!(open.get(), Some(1));
  stage.pass();
  assert!(stage.paints(DARK_ANSWER));
  assert!(!stage.paints(GPU_ANSWER), "the other one closed");

  press(&mut stage, DARK);
  assert_eq!(open.get(), None, "all of them closed");
  stage.pass();
  assert!(!stage.paints(DARK_ANSWER));
}

#[test]
fn any_number_are_open_in_an_accordion_of_several() {
  let open: Signal<Vec<u8>> = Signal::new(Vec::new());
  let read = open.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(Accordion::multiple(read.clone(), items()).style(AccordionPart::Root, |s| s.width(360.0)))
      .into()
  });
  press(&mut stage, GPU);
  press(&mut stage, THEME);
  assert_eq!(open.get(), [0, 2]);
  assert!(stage.paints(GPU_ANSWER) && stage.paints(THEME_ANSWER));
  press(&mut stage, GPU);
  assert_eq!(open.get(), [2]);
  stage.pass();
  assert!(!stage.paints(GPU_ANSWER) && stage.paints(THEME_ANSWER));
}

#[test]
fn a_controlled_accordion_shows_what_the_app_says_and_tells_it_of_a_press() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let tell = told.clone();
  let mut stage = Stage::new(move |_| {
    let tell = tell.clone();
    Column::new()
      .padding(8.0)
      .child(Accordion::controlled([1], items(), move |value| {
        tell.lock().unwrap().push(value)
      }))
      .into()
  });
  assert!(stage.paints(DARK_ANSWER));
  press(&mut stage, GPU);
  press(&mut stage, DARK);
  assert_eq!(*told.lock().unwrap(), [0, 1]);
  // The app opened nothing and closed nothing.
  assert!(stage.paints(DARK_ANSWER) && !stage.paints(GPU_ANSWER));
}

#[test]
fn each_header_is_a_stop_of_tab_and_the_arrows_move_between_them() {
  let open = Signal::new(None);
  let mut stage = faq(&open);
  // Tab reaches the first header, with the ring of the focus, and Enter opens it.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("Enter");
  assert_eq!(open.get(), Some(0));
  // Down moves to the next header, and Space opens it.
  stage.key("ArrowDown");
  stage.key(" ");
  assert_eq!(open.get(), Some(1));
  // Up goes back, and from the first it goes round to the last.
  stage.key("ArrowUp");
  stage.key("ArrowUp");
  stage.key(" ");
  assert_eq!(open.get(), Some(2));
  // Tab goes on from a header to the next one, as a web accordion's does: from the last it leaves.
  stage.key("Home");
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(open.get(), Some(1));
}

/// Two questions, the first of which holds a button that counts its presses in `pressed`.
fn with_a_button(open: &Signal<Option<u8>>, pressed: &Arc<Mutex<u32>>) -> Stage {
  let (read, count) = (open.clone(), pressed.clone());
  Stage::new(move |_| {
    let count = count.clone();
    let inside = Column::new().child(
      Button::new("Read more")
        .tab_index(0)
        .on_click(move |_| *count.lock().unwrap() += 1),
    );
    Column::new()
      .padding(8.0)
      .child(Accordion::new(
        read.clone(),
        [
          AccordionItem::new(0, GPU, inside),
          AccordionItem::text(1, DARK, DARK_ANSWER),
        ],
      ))
      .into()
  })
}

#[test]
fn the_arrows_move_between_the_headers_past_what_an_open_item_holds() {
  let (open, pressed) = (Signal::new(Some(0)), Arc::new(Mutex::new(0)));
  let mut stage = with_a_button(&open, &pressed);
  // From the first header Down goes to the second, not to the button the open item holds: Space opens it.
  stage.key("Tab");
  stage.key("ArrowDown");
  stage.key(" ");
  assert_eq!(*pressed.lock().unwrap(), 0);
  assert_eq!(open.get(), Some(1));

  // On the button in the item the arrows are not the accordion's: the focus stays on it.
  let (open, pressed) = (Signal::new(Some(0)), Arc::new(Mutex::new(0)));
  let mut stage = with_a_button(&open, &pressed);
  stage.key("Tab");
  stage.key("Tab");
  stage.key("ArrowDown");
  stage.key("ArrowUp");
  stage.key("Enter");
  assert_eq!(*pressed.lock().unwrap(), 1);
  assert_eq!(open.get(), Some(0));
}

#[test]
fn the_controls_in_a_closed_item_are_not_stops_of_tab() {
  // Open, the button in the item is the stop after its header.
  let (open, pressed) = (Signal::new(Some(0)), Arc::new(Mutex::new(0)));
  let mut stage = with_a_button(&open, &pressed);
  stage.key("Tab");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(*pressed.lock().unwrap(), 1);

  // Closed, it is not there, and Tab goes from the first header to the second.
  let (open, pressed) = (Signal::new(None), Arc::new(Mutex::new(0)));
  let mut stage = with_a_button(&open, &pressed);
  assert!(!stage.paints("Read more"));
  stage.key("Tab");
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(*pressed.lock().unwrap(), 0);
  assert_eq!(open.get(), Some(1));
}

#[test]
fn a_disabled_item_or_accordion_takes_nothing() {
  let open = Signal::new(None);
  let read = open.clone();
  let mut stage = Stage::new(move |_| {
    let [gpu, dark, theme] = items();
    Column::new()
      .padding(8.0)
      .child(Accordion::new(read.clone(), [gpu, dark.disabled(true), theme]))
      .into()
  });
  press(&mut stage, DARK);
  assert_eq!(open.get(), None);
  // Tab passes it by.
  stage.key("Tab");
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(open.get(), Some(2));

  let open = Signal::new(None);
  let read = open.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(Accordion::new(read.clone(), items()).disabled(true))
      .into()
  });
  press(&mut stage, GPU);
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(open.get(), None);
}

#[test]
fn an_item_opens_and_closes_with_its_height() {
  let open = Signal::new(None);
  let mut stage = faq(&open);
  let closed = top(&stage, DARK);
  open.set(Some(0));
  stage.pass();
  let opened = top(&stage, DARK);
  assert!(
    opened > closed + 14.0,
    "the item under it moved down by what opened: {closed} -> {opened}"
  );

  // Closing, the item under it rises with the panel, through every height between.
  stage.pass();
  open.set(None);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(40));
  let on_the_way = top(&stage, DARK);
  assert!(
    on_the_way > closed + 0.5 && on_the_way < opened - 0.5,
    "on the way: {closed} < {on_the_way} < {opened}"
  );
  stage.pass();
  assert!(close(top(&stage, DARK), closed));
  assert!(!stage.paints(GPU_ANSWER), "closed, what it held is gone");

  // Opening, it falls again from where it is to the height of what opens, which the layout reads as it goes.
  open.set(Some(0));
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(40));
  let on_the_way = top(&stage, DARK);
  assert!(
    on_the_way > closed + 0.5 && on_the_way < opened - 0.5,
    "on the way: {closed} < {on_the_way} < {opened}"
  );
  stage.pass();
  assert!(close(top(&stage, DARK), opened));
}

#[test]
fn with_motion_reduced_an_item_opens_at_once() {
  let open = Signal::new(None);
  let mut stage = faq(&open);
  let closed = top(&stage, DARK);
  open.set(Some(0));
  stage.pass();
  let opened = top(&stage, DARK);
  open.set(None);
  stage.pass();
  stage.pass();
  assert!(close(top(&stage, DARK), closed));

  stage.app.theme().set_reduced_motion(Some(true));
  stage.pass();
  open.set(Some(0));
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(10));
  assert!(close(top(&stage, DARK), opened), "at once");
}

#[test]
fn a_rule_of_the_app_styles_a_part_of_every_item() {
  let open = Signal::new(Some(0));
  let read = open.clone();
  let stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(
        Accordion::new(read.clone(), items())
          .style(AccordionPart::Label, |s| s.font_size(16.0))
          .style(AccordionPart::Content, |s| s.foreground(PaletteColor::Primary)),
      )
      .into()
  });
  assert_eq!(stage.text(DARK).1, 16.0);
  assert_eq!(stage.text(GPU_ANSWER).0, light(PaletteColor::Primary));
}

/// The collapsible of the board: 280 wide, its header pressed by the id `crates`.
fn crates(open: &Signal<bool>, disabled: bool) -> Stage {
  let read = open.clone();
  Stage::new(move |_| {
    let set = read.clone();
    Column::new()
      .padding(8.0)
      .child(
        Collapsible::text("3 crates in this workspace", "lurq, lurq_macros, demo")
          .open(read.get())
          .disabled(disabled)
          .id("crates")
          .style(CollapsiblePart::Root, |s| s.width(280.0))
          .on_change(move |next| set.set(next)),
      )
      .into()
  })
}

#[test]
fn a_collapsible_is_a_header_with_a_ghost_button_at_its_end() {
  let open = Signal::new(true);
  let mut stage = crates(&open, false);
  let (x, y, width, height) = stage.bounds("crates");
  // As wide as the collapsible, as high as the button of 32 at its end.
  assert_eq!((width, height), (280.0, 32.0));
  assert_eq!(
    stage.text("3 crates in this workspace"),
    (light(PaletteColor::Foreground), 13.0)
  );
  assert_eq!(stage.text(CHEVRONS_VERTICAL), (light(PaletteColor::Foreground), 16.0));
  let icon = stage.text_quad(CHEVRONS_VERTICAL).unwrap();
  assert!(
    close(icon.x + 8.0 + icon.width, x + 280.0),
    "in the middle of the button at the end"
  );
  // What it opens is 8 under the header.
  assert!(close(top(&stage, "lurq, lurq_macros, demo") - (y + 32.0), 8.0));
  assert_eq!(stage.text("lurq, lurq_macros, demo").1, 14.0);

  // The button is filled with the highlight under the pointer.
  let (cx, cy) = (icon.x + icon.width / 2.0, icon.y + icon.height / 2.0);
  stage.tree.mouse_move(cx, cy);
  stage.pass();
  let lit = stage.quads().into_iter().any(|quad| {
    matches!(quad.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Highlight))
      && (quad.width, quad.height) == (32.0, 32.0)
  });
  assert!(lit, "the ghost button is lit");
}

#[test]
fn a_press_of_the_header_asks_for_the_other_state() {
  let open = Signal::new(false);
  let mut stage = crates(&open, false);
  assert!(!stage.paints("lurq, lurq_macros, demo"));
  // The keyboard: Tab reaches the header, Space opens it, Enter closes it.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key(" ");
  assert!(open.get());
  assert!(stage.paints("lurq, lurq_macros, demo"));
  stage.key("Enter");
  assert!(!open.get());
  stage.click("crates");
  assert!(open.get());
}

#[test]
fn a_disabled_collapsible_takes_nothing() {
  let open = Signal::new(false);
  let mut stage = crates(&open, true);
  stage.click("crates");
  stage.key("Tab");
  stage.key(" ");
  assert!(!open.get());
}

#[test]
fn a_forced_focus_draws_the_ring_around_the_header() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Collapsible::text("Details", "More").force(Some(Interaction::FocusVisible)))
      .into()
  });
  stage.pass();
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_theme_gives_the_collapsible_another_anatomy_and_it_still_opens() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let app = App::new();
  // The header says whether it is open in words, with no icon; the panel is the default one.
  let renderer: CollapsibleRenderer = Arc::new(|mut view: CollapsibleView| {
    let word = if view.open { "Hide" } else { "Show" };
    let trigger = view.trigger().child(view.part(CollapsiblePart::Label).text(word));
    let panel = view.panel();
    Column::new().child(trigger).child(panel).into()
  });
  app.theme().set_renderer::<Collapsible>(renderer);
  let mut stage = Stage::in_app(app, move |_| {
    let told = told.clone();
    Row::new()
      .padding(8.0)
      .child(
        Collapsible::text("Details", "More")
          .id("details")
          .on_change(move |next| told.lock().unwrap().push(next)),
      )
      .into()
  });
  assert!(stage.paints("Show"));
  assert!(!stage.paints(CHEVRONS_VERTICAL));
  stage.click("details");
  assert_eq!(*asked.lock().unwrap(), [true]);
}
