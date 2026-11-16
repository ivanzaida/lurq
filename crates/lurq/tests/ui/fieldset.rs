//! A fieldset: its edge, its legend and the fields under it where the board has them, and a disabled one that
//! takes nothing. An error summary: its box, its title and its lines, a press and a key of a line that focus the
//! field, one made from a form, and a theme's rule.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::theme::{OpacityLevel, PaletteColor, ThemePalette},
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{
    Button, Case, ErrorSummary, ErrorSummaryPart, Field, FieldError, Fieldset, FieldsetPart, FieldsetState,
    FieldsetVariant, Input, Opacity,
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
  // With no room around them: a summary is 400 wide where it has 400, and the stage is 400.
  let mut stage = Stage::new(|_| {
    Column::new()
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

#[cfg(feature = "form")]
#[test]
fn a_summary_of_a_form_lists_its_errors_once_a_submit_was_attempted() {
  use lurq::components::{FormData, FormHandle, FormOptions, ValidationResult};

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

#[test]
fn a_summary_adds_errors_after_the_first_and_a_line_has_its_own_id() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(400.0)
      .child(
        ErrorSummary::new([FieldError::new("Display name", "Display name is required").id("name-error")])
          .error(FieldError::new("Email", "Email must include a domain"))
          .errors([FieldError::new("Password", "Password is too short").key("password")]),
      )
      .into()
  });
  assert!(stage.paints("3 fields need attention"));
  for error in [
    "Display name is required",
    "Email must include a domain",
    "Password is too short",
  ] {
    assert!(stage.paints(error), "{error:?}");
  }
  let (_, y, _, height) = stage.bounds("name-error");
  assert!(height > 0.0 && y > 0.0, "the first line has its id");
}

/// The quad of the edge of a group box's body: the one edge painted that is `width` wide.
fn box_edge(stage: &Stage, width: f32) -> Quad {
  stage
    .quads()
    .into_iter()
    .find(|quad| quad.border.is_some() && quad.width == width)
    .expect("the group box should have an edge around what it holds")
}

#[test]
fn a_group_box_is_the_board_s_small_legend_over_a_box_of_what_it_holds() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(
        Fieldset::new("Startup")
          .variant(FieldsetVariant::GroupBox)
          .child(Row::new().id("first").height(16.0).width(106.0))
          .child(Row::new().id("second").height(16.0).width(129.0))
          .child(Row::new().id("third").height(16.0).width(138.0))
          .id("group"),
      )
      .into()
  });
  // 250 wide, with no edge of its own: the legend is over the box.
  let (x, y, width, height) = stage.bounds("group");
  assert_eq!(width, 250.0);
  assert!(stage.border("group").is_none(), "the edge is around what it holds");
  let title = stage.text_quad("Startup").unwrap();
  assert_eq!((title.x, title.y), (x, y));
  assert_eq!(stage.text("Startup"), (light(PaletteColor::MutedForeground), 12.0));

  // The box: 6 under the legend, an edge of 1 in the border's color with the medium radius, padded 12, its
  // children 10 apart.
  let edge = box_edge(&stage, 250.0);
  assert!((edge.y - (title.y + title.height + 6.0)).abs() < 0.5);
  let side = edge.border.and_then(|border| border.top).unwrap();
  assert_eq!((side.width, side.color), (1.0, light(PaletteColor::Border)));
  assert!(
    edge.border_radius.is_some_and(|radius| radius.top_left == 6.0),
    "radius-md"
  );
  let (first, second, third) = (stage.bounds("first"), stage.bounds("second"), stage.bounds("third"));
  assert_eq!((first.0 - edge.x, first.1 - edge.y), (12.0, 12.0));
  assert_eq!(second.1 - (first.1 + first.3), 10.0);
  assert_eq!(third.1 - (second.1 + second.3), 10.0);
  // Three rows of 16 with 10 between them and 12 around: the board's 92, and the whole is its 113.
  assert_eq!(edge.height, 92.0);
  assert!(
    (y + height - (edge.y + edge.height)).abs() < 0.01,
    "the box ends the group"
  );
  assert!(
    (height - 113.0).abs() < 1.5,
    "the board's group box is 113 high: {height}"
  );
}

#[test]
fn a_group_box_names_its_group_wraps_a_long_legend_and_is_disabled_as_a_fieldset_is() {
  let presses = Arc::new(AtomicUsize::new(0));
  let count = presses.clone();
  let legend = "Everything this app does when the computer starts and a person signs in";
  let mut stage = Stage::new(move |_| {
    let count = count.clone();
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(
        Fieldset::new(legend)
          .variant(FieldsetVariant::GroupBox)
          .description("Applies the next time you sign in.")
          .child(Button::new("Open at login").id("open").on_click(move |_| {
            count.fetch_add(1, Ordering::SeqCst);
          }))
          .disabled(true)
          .id("group"),
      )
      .into()
  });
  // A legend longer than the box goes on to the next line, in the box's width; the description is under it in 12.
  let (x, _, width, _) = stage.bounds("group");
  assert_eq!(width, 250.0);
  let wrapped = stage.text_quad(legend).expect("the legend is painted");
  assert!(wrapped.height > 25.0, "it goes on to the next line: {}", wrapped.height);
  for quad in stage.quads() {
    if matches!(&quad.content, QuadContent::Text { .. }) {
      assert!(quad.x + quad.width <= x + width + 0.5, "no text is past the box");
    }
  }
  assert_eq!(
    stage.text("Applies the next time you sign in."),
    (light(PaletteColor::MutedForeground), 12.0)
  );
  // Disabled, nothing in it is pressed or focused.
  stage.click("open");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(presses.load(Ordering::SeqCst), 0);
  let theme = stage.app.theme().clone();
  let case = Case::new()
    .variant(FieldsetVariant::GroupBox)
    .state(FieldsetState::Disabled, true);
  assert_eq!(
    theme
      .resolve::<Fieldset>(&case, None)
      .part(FieldsetPart::Root)
      .rest()
      .opacity,
    Some(Opacity::Level(OpacityLevel::Disabled))
  );
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn a_group_box_is_a_group_called_by_its_legend_to_tooling() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(Fieldset::new("Startup").variant(FieldsetVariant::GroupBox).id("group"))
      .into()
  });
  let said = |stage: &mut Stage, key: &str| {
    stage
      .tree
      .get_element_by_id_mut("group")
      .and_then(|element| element.description(key))
  };
  assert_eq!(said(&mut stage, "a11y_role").as_deref(), Some("group"));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Startup"));
}

#[test]
fn a_full_width_group_box_fills_its_room() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(
        Fieldset::new("Startup")
          .variant(FieldsetVariant::GroupBox)
          .full_width(true)
          .id("group"),
      )
      .into()
  });
  assert_eq!(stage.bounds("group").2, 384.0);
}
