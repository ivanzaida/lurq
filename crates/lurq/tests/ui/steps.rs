//! Steps: the board's marks, titles and lines in a row and in a column, what a
//! press and the keyboard select, a step in error, and a theme's other anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::quad::Quad,
  node::color::Color,
  ui::{
    Case, Interaction, Opacity, Step, Steps, StepsOrientation, StepsPart, StepsRenderer, StepsState, StepsView, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The color of the edge of the element with `id`, where it has one.
fn edge(stage: &mut Stage, id: &str) -> Option<Color> {
  stage
    .border(id)
    .and_then(|quad: Quad| quad.border)
    .and_then(|border| border.top)
    .map(|side| side.color)
}

/// What a test is told of each press.
fn told() -> (Arc<Mutex<Vec<usize>>>, Arc<Mutex<Vec<usize>>>) {
  let asked = Arc::new(Mutex::new(Vec::new()));
  (asked.clone(), asked)
}

const WIZARD: [&str; 4] = ["Account", "Billing", "Workspace", "Finish"];

#[test]
fn steps_in_a_row_are_marks_with_titles_and_lines_up_to_the_current_one() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(460.0)
      .child(Steps::new(1, ["Account", "Workspace", "Finish"]).id("steps"))
      .into()
  });
  let (_, _, width, _) = stage.bounds("steps");
  assert_eq!(
    width, 460.0,
    "the whole is as wide as it is given, its lines taking what the steps leave"
  );

  for index in 0..3 {
    let (.., width, height) = stage.bounds(&format!("steps-{index}-marker"));
    assert_eq!((width, height), (24.0, 24.0), "mark {index}");
  }
  // Done and current: filled with the primary color. To come: an edge in an input's color, nothing filled.
  assert_eq!(stage.fill_color("steps-0-marker"), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color("steps-1-marker"), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color("steps-2-marker"), None);
  assert_eq!(edge(&mut stage, "steps-2-marker"), Some(light(PaletteColor::Input)));

  // A step done has a check in place of its number; the current one and the one to come have theirs.
  assert!(!stage.paints("1"), "the check, not the number");
  assert_eq!(stage.text("2"), (light(PaletteColor::PrimaryForeground), 12.0));
  assert_eq!(stage.text("3"), (light(PaletteColor::MutedForeground), 12.0));

  // The titles in the label's type, muted while the step is to come.
  assert_eq!(stage.text("Account"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(stage.text("Workspace"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(stage.text("Finish"), (light(PaletteColor::MutedForeground), 13.0));

  // A line of 1 from one step to the next: in the primary color up to the current step, the border's after it.
  assert_eq!(stage.fill_color("steps-0-line"), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color("steps-1-line"), Some(light(PaletteColor::Border)));
  assert!(
    stage.tree.get_element_by_id_mut("steps-2-line").is_none(),
    "no line after the last step"
  );

  let (mark_x, mark_y, ..) = stage.bounds("steps-0-marker");
  let title = stage.text_quad("Account").unwrap();
  assert!(
    (title.x - (mark_x + 24.0) - 8.0).abs() < 0.5,
    "the title is 8 beside the mark"
  );
  let (line_x, line_y, _, line_height) = stage.bounds("steps-0-line");
  assert_eq!(line_height, 1.0);
  assert!(
    (line_x - (title.x + title.width) - 12.0).abs() < 0.5,
    "the line is 12 after the title"
  );
  assert!(
    (line_y + 0.5 - (mark_y + 12.0)).abs() < 0.5,
    "the line is level with the middle of the mark"
  );
  let (next_x, ..) = stage.bounds("steps-1-marker");
  let (line_x, _, line_width, _) = stage.bounds("steps-0-line");
  assert!(
    (next_x - (line_x + line_width) - 12.0).abs() < 0.5,
    "and ends 12 before the next mark"
  );
}

#[test]
fn steps_in_a_column_have_a_line_down_from_each_mark_to_the_next() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .child(
        Steps::new(
          1,
          [
            Step::new("Account").description("Name and email"),
            Step::new("Billing").description("Card and address"),
            Step::new("Finish"),
          ],
        )
        .orientation(StepsOrientation::Vertical)
        .id("steps"),
      )
      .into()
  });
  let (x0, y0, ..) = stage.bounds("steps-0-marker");
  let (x1, y1, ..) = stage.bounds("steps-1-marker");
  let (x2, y2, ..) = stage.bounds("steps-2-marker");
  assert_eq!((x0, x1), (x1, x2), "the marks one under the other");
  assert!(y0 < y1 && y1 < y2);

  for (index, color, (top, bottom)) in [
    (0, PaletteColor::Primary, (y0, y1)),
    (1, PaletteColor::Border, (y1, y2)),
  ] {
    let id = format!("steps-{index}-line");
    let (x, y, width, height) = stage.bounds(&id);
    assert_eq!(width, 1.0, "{id}");
    assert!(
      (x + 0.5 - (x0 + 12.0)).abs() < 0.5,
      "{id}: under the middle of the mark"
    );
    assert!(y >= top + 24.0 && y + height <= bottom, "{id}: between the two marks");
    assert!(height > 0.0, "{id}: seen");
    assert_eq!(stage.fill_color(&id), Some(light(color)), "{id}");
  }
  assert!(stage.tree.get_element_by_id_mut("steps-2-line").is_none());

  // The description under the title, muted and smaller.
  let title = stage.text_quad("Billing").unwrap();
  let description = stage.text_quad("Card and address").unwrap();
  assert!(description.y > title.y);
  assert!((description.x - title.x).abs() < 0.5);
  assert_eq!(
    stage.text("Card and address"),
    (light(PaletteColor::MutedForeground), 12.0)
  );
}

#[test]
fn a_press_goes_back_to_a_step_done_and_not_on_to_one_to_come() {
  let (asked, record) = told();
  let mut stage = Stage::new(move |_| {
    let record = record.clone();
    Steps::new(2, WIZARD)
      .id("steps")
      .on_select(move |index| record.lock().unwrap().push(index))
      .into()
  });
  for index in 0..4 {
    stage.click(&format!("steps-{index}"));
  }
  assert_eq!(
    *asked.lock().unwrap(),
    [0, 1],
    "the steps done; not the current one, not one to come"
  );

  // Where the app lets steps be taken in any order, a step to come is pressed too.
  let (asked, record) = told();
  let mut stage = Stage::new(move |_| {
    let record = record.clone();
    Steps::new(2, WIZARD)
      .id("steps")
      .ahead(true)
      .on_select(move |index| record.lock().unwrap().push(index))
      .into()
  });
  stage.click("steps-3");
  stage.click("steps-0");
  assert_eq!(*asked.lock().unwrap(), [3, 0]);

  // Disabled, nothing is.
  let (asked, record) = told();
  let mut stage = Stage::new(move |_| {
    let record = record.clone();
    Steps::new(2, WIZARD)
      .id("steps")
      .disabled(true)
      .on_select(move |index| record.lock().unwrap().push(index))
      .into()
  });
  stage.click("steps-0");
  stage.key("Tab");
  stage.key("Enter");
  assert!(asked.lock().unwrap().is_empty());
  assert!(!stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_press_sets_the_signal_of_the_current_step() {
  let at = Signal::new(2usize);
  let bound = at.clone();
  let mut stage = Stage::new(move |_| Steps::new(&bound, WIZARD).id("steps").pressable(true).into());
  assert_eq!(stage.fill_color("steps-1-line"), Some(light(PaletteColor::Primary)));
  stage.click("steps-0");
  assert_eq!(at.get(), 0);
  stage.pass();
  // Back at the first step, the second is to come again.
  assert_eq!(stage.fill_color("steps-1-marker"), None);
  assert_eq!(stage.fill_color("steps-0-line"), Some(light(PaletteColor::Border)));
  assert_eq!(stage.text("Billing").0, light(PaletteColor::MutedForeground));
}

#[test]
fn steps_that_are_only_read_take_neither_a_press_nor_the_focus() {
  let at = Signal::new(2usize);
  let bound = at.clone();
  let mut stage = Stage::new(move |_| Steps::new(&bound, WIZARD).id("steps").into());
  stage.click("steps-0");
  stage.key("Tab");
  assert_eq!(at.get(), 2);
  assert!(!stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn tab_enters_at_the_current_step_and_the_arrows_move_between_the_steps_pressed() {
  let (asked, record) = told();
  let mut stage = Stage::new(move |_| {
    let record = record.clone();
    Row::new()
      .padding(8.0)
      .child(
        Steps::new(2, WIZARD)
          .id("steps")
          .on_select(move |index| record.lock().unwrap().push(index)),
      )
      .into()
  });
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)), "the current step is focused");
  stage.key("Enter");
  assert!(
    asked.lock().unwrap().is_empty(),
    "a press of the current step selects nothing"
  );
  stage.key("ArrowLeft");
  stage.key("Enter");
  stage.key("ArrowLeft");
  stage.key(" ");
  assert_eq!(*asked.lock().unwrap(), [1, 0]);

  // In a column, Up and Down move.
  let (asked, record) = told();
  let mut stage = Stage::new(move |_| {
    let record = record.clone();
    Row::new()
      .padding(8.0)
      .child(
        Steps::new(2, WIZARD)
          .orientation(StepsOrientation::Vertical)
          .on_select(move |index| record.lock().unwrap().push(index)),
      )
      .into()
  });
  stage.key("Tab");
  stage.key("ArrowUp");
  stage.key("ArrowUp");
  stage.key(" ");
  assert_eq!(*asked.lock().unwrap(), [0]);
}

#[test]
fn a_forced_focus_rings_the_step_tab_would_enter_at() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(
        Steps::new(1, WIZARD)
          .pressable(true)
          .force(Some(Interaction::FocusVisible)),
      )
      .into()
  });
  stage.pass();
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_step_in_error_says_so_in_its_mark_and_its_title() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(460.0)
      .child(
        Steps::new(
          2,
          [
            Step::new("Account"),
            Step::new("Billing").error(true),
            Step::new("Workspace").icon(icons::USER),
            Step::new("Finish"),
          ],
        )
        .id("steps"),
      )
      .into()
  });
  assert_eq!(
    stage.fill_color("steps-1-marker"),
    Some(light(PaletteColor::Destructive))
  );
  assert_eq!(stage.text("Billing").0, light(PaletteColor::Destructive));
  assert!(!stage.paints("2"), "a cross in place of the number");
  assert!(!stage.paints("3"), "the step's icon in place of its number");
  assert!(stage.paints("4"));
}

#[test]
fn disabled_steps_are_dimmed() {
  let stage = Stage::new(|_| Steps::new(1, WIZARD).disabled(true).into());
  let theme = stage.app.theme().clone();
  let case = Case::new()
    .variant(StepsOrientation::Horizontal)
    .state(StepsState::Disabled, true);
  let root = theme.resolve::<Steps>(&case, None).part(StepsPart::Root).rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
}

#[test]
fn a_theme_gives_steps_another_anatomy_and_they_still_select() {
  let (asked, record) = told();
  let app = App::new();
  // Each step its name in words, as tooling reads it: other nodes, the same behavior.
  let renderer: StepsRenderer = Arc::new(|view: StepsView| {
    let mut row = Row::new().spacing(8.0);
    for index in 0..view.steps.len() {
      let words = view.step_part(index, StepsPart::Title).text(&view.step_name(index));
      row = row.child(view.step(index, [words.into()]));
    }
    row.into()
  });
  app.theme().set_renderer::<Steps>(renderer);
  let mut stage = Stage::in_app(app, move |_| {
    let record = record.clone();
    Steps::new(1, ["Account", "Billing", "Finish"])
      .id("steps")
      .on_select(move |index| record.lock().unwrap().push(index))
      .into()
  });
  assert!(stage.paints("Step 1 of 3, Account, done"));
  assert!(stage.paints("Step 2 of 3, Billing, current"));
  assert!(stage.paints("Step 3 of 3, Finish, to come"));
  assert!(!stage.paints("Account"));
  stage.click("steps-0");
  assert_eq!(*asked.lock().unwrap(), [0]);
}
