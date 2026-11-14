//! A fieldset: its edge, its legend and the fields under it where the board has them, and a disabled one that
//! takes nothing. An error summary: its box, its title and its lines, a press and a key of a line that focus the
//! field, one made from a form, and a theme's rule.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::theme::{OpacityLevel, PaletteColor, ThemePalette},
  components::{Column, FormData, FormHandle, FormOptions, Row, ValidationResult},
  core::{ElementRef, Signal},
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{
    Button, Case, ErrorSummary, ErrorSummaryPart, Field, FieldError, Fieldset, FieldsetPart, FieldsetState, Input,
    Opacity,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The width and the color of the edge of the element with `id`.
fn edge(stage: &mut Stage, id: &str) -> (f32, Color) {
  let quad = stage.border(id).unwrap_or_else(|| panic!("#{id} should have an edge"));
  let side = quad.border.and_then(|border| border.top).unwrap();
  (side.width, side.color)
}

/// The id of what has the focus.
fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// A press of the pointer at the middle of `quad`.
fn press_at(stage: &mut Stage, quad: &Quad) {
  let (x, y) = (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  stage.pass();
}

#[test]
fn a_fieldset_is_the_boards_box_with_its_legend_over_its_fields() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(500.0, 400.0)
      .padding(8.0)
      .child(
        Fieldset::new("Profile")
          .description("Shown to people you share a workspace with.")
          .field(Field::new("Display name", Input::new(Signal::new(String::new())).full_width(true)).id("name"))
          .child(Row::new().id("actions").height(36.0).child(Button::new("Save profile")))
          .id("fieldset"),
      )
      .into()
  });
  let (x, y, width, _) = stage.bounds("fieldset");
  assert_eq!(width, 400.0);
  assert_eq!(edge(&mut stage, "fieldset"), (1.0, light(PaletteColor::Border)));
  let border = stage.border("fieldset").unwrap();
  assert!(
    border.border_radius.is_some_and(|radius| radius.top_left == 8.0),
    "radius-lg"
  );

  // The legend in the corner of the padding of 20: its title in 15, its description 4 under it in 13.
  let title = stage.text_quad("Profile").unwrap();
  assert_eq!((title.x - x, title.y - y), (20.0, 20.0));
  assert_eq!(stage.text("Profile"), (light(PaletteColor::Foreground), 15.0));
  let description = stage.text_quad("Shown to people you share a workspace with.").unwrap();
  assert_eq!(
    stage.text("Shown to people you share a workspace with."),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  assert!((description.y - (title.y + title.height + 4.0)).abs() < 0.5);

  // A field is as wide as the room inside the fieldset, 16 under the legend; what follows is 16 under it.
  let field = stage.bounds("name");
  assert_eq!((field.0 - x, field.2), (20.0, 360.0));
  assert!((field.1 - (description.y + description.height + 16.0)).abs() < 0.5);
  let actions = stage.bounds("actions");
  assert_eq!(actions.1 - (field.1 + field.3), 16.0);
}

#[test]
fn a_disabled_fieldset_takes_nothing_in_it_and_is_dimmed() {
  let presses = Arc::new(AtomicUsize::new(0));
  let count = presses.clone();
  let mut stage = Stage::new(move |_| {
    let count = count.clone();
    Column::new()
      .padding(8.0)
      .child(
        Fieldset::new("Billing")
          .child(Button::new("Change plan").id("change").on_click(move |_| {
            count.fetch_add(1, Ordering::SeqCst);
          }))
          .disabled(true),
      )
      .into()
  });
  stage.click("change");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(presses.load(Ordering::SeqCst), 0);
  assert_eq!(focused(&stage), None, "Tab passes everything in it by");

  let theme = stage.app.theme().clone();
  let root = theme
    .resolve::<Fieldset>(&Case::new().state(FieldsetState::Disabled, true), None)
    .part(FieldsetPart::Root)
    .rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
}

/// A summary over two inputs, each in a row a ref is on.
fn summary_scene(name: ElementRef, email: ElementRef) -> Stage {
  Stage::new(move |_| {
    Column::new()
      .size(500.0, 300.0)
      .padding(8.0)
      .spacing(12.0)
      .child(
        ErrorSummary::new([
          FieldError::new("Display name", "Display name is required").focuses(&name),
          FieldError::new("Email", "Email must include a domain").focuses(&email),
        ])
        .id("summary"),
      )
      .child(
        Row::new()
          .ref_element(name.clone())
          .child(Input::new(Signal::new(String::new())).id("name")),
      )
      .child(
        Row::new()
          .ref_element(email.clone())
          .child(Input::new(Signal::new("ivan@".to_owned())).id("email")),
      )
      .into()
  })
}

#[test]
fn an_error_summary_is_the_boards_box_with_a_title_and_a_line_for_each_field() {
  let mut stage = summary_scene(ElementRef::new(), ElementRef::new());
  let (x, y, width, _) = stage.bounds("summary");
  assert_eq!(width, 400.0);
  assert_eq!(stage.fill_color("summary"), Some(light(PaletteColor::DestructiveMuted)));
  assert_eq!(edge(&mut stage, "summary"), (1.0, light(PaletteColor::Destructive)));
  let fill = stage.fill("summary").unwrap();
  assert!(
    fill.border_radius.is_some_and(|radius| radius.top_left == 6.0),
    "radius-md"
  );

  // The title counts the fields, in the destructive color; it is 16 + 10 after the padding of 14.
  let title = stage.text_quad("2 fields need attention").unwrap();
  assert_eq!(
    stage.text("2 fields need attention"),
    (light(PaletteColor::Destructive), 13.0)
  );
  assert_eq!((title.x - x, title.y - y), (14.0 + 16.0 + 10.0, 12.0));
  // Each line is its message in the foreground, after a dot of 4 in the destructive color 8 before it.
  let message = stage.text_quad("Display name is required").unwrap();
  assert_eq!(
    stage.text("Display name is required"),
    (light(PaletteColor::Foreground), 13.0)
  );
  assert_eq!(message.x - title.x, 4.0 + 8.0);
  let bullet = stage.quads().into_iter().find(|quad| {
    (quad.width, quad.height) == (4.0, 4.0)
      && matches!(quad.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Destructive))
  });
  assert!(bullet.is_some_and(|bullet| bullet.x == title.x), "the dot");
  let second = stage.text_quad("Email must include a domain").unwrap();
  assert!(second.y > message.y + message.height);
}

#[test]
fn a_press_or_the_keyboard_on_a_line_focuses_its_field() {
  let (name, email) = (ElementRef::new(), ElementRef::new());
  let mut stage = summary_scene(name, email);
  let line = stage.text_quad("Email must include a domain").unwrap();
  press_at(&mut stage, &line);
  assert_eq!(focused(&stage).as_deref(), Some("email"));

  // Shift+Tab goes back from the email to the name, the second line and the first; Enter on it focuses the name.
  for _ in 0..3 {
    stage
      .tree
      .key_down("Tab".to_owned(), "Tab".to_owned(), true, false, false);
    stage.pass();
  }
  assert!(focused(&stage).is_none(), "a line has the focus, and no id");
  stage.key("Enter");
  assert_eq!(focused(&stage).as_deref(), Some("name"));
}

#[test]
fn a_summary_with_no_errors_draws_nothing_and_one_error_is_one_field() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(ErrorSummary::new([]))
      .child(ErrorSummary::new([FieldError::new("Email", "Email must include a domain")]).id("one"))
      .child(
        ErrorSummary::new([FieldError::new("Email", "Bad")])
          .title("Corrige les champs")
          .style(ErrorSummaryPart::Title, |s| s.font_size(16.0)),
      )
      .into()
  });
  assert!(stage.paints("1 field needs attention"));
  assert!(!stage.paints("0 fields need attention"));
  assert_eq!(stage.text("Corrige les champs").1, 16.0);
  let (_, _, width, _) = stage.bounds("one");
  assert_eq!(width, 400.0);
}

#[test]
fn a_summary_of_a_form_lists_its_errors_once_a_submit_was_attempted() {
  let form = FormHandle::new(
    FormOptions::new()
      .field("name", "")
      .field("email", "ivan@")
      .field("bio", "")
      .validate_string("name", |value, _| {
        if value.is_empty() {
          ValidationResult::invalid("Display name is required")
        } else {
          ValidationResult::valid()
        }
      })
      .validate_string("email", |value, _| {
        if value.contains('.') {
          ValidationResult::valid()
        } else {
          ValidationResult::invalid("Email must include a domain")
        }
      }),
  );
  let shown = form.clone();
  let (name, email) = (ElementRef::new(), ElementRef::new());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(500.0, 300.0)
      .padding(8.0)
      .child(ErrorSummary::form(
        &shown,
        [("email", "Email", &email), ("name", "Display name", &name)],
      ))
      .child(
        Row::new()
          .ref_element(email.clone())
          .child(Input::new(shown.string("email")).id("email")),
      )
      .child(
        Row::new()
          .ref_element(name.clone())
          .child(Input::new(shown.string("name")).id("name")),
      )
      .into()
  });
  assert!(!stage.paints("Email must include a domain"), "nothing before a submit");
  form.submit(FormData::new());
  stage.pass();
  assert!(stage.paints("2 fields need attention"));
  // In the order the app gave the fields.
  let (email_line, name_line) = (
    stage.text_quad("Email must include a domain").unwrap(),
    stage.text_quad("Display name is required").unwrap(),
  );
  assert!(email_line.y < name_line.y);
  press_at(&mut stage, &name_line);
  assert_eq!(focused(&stage).as_deref(), Some("name"));
}
