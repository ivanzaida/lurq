//! The form controls of `lurq::components` as the design system draws them: a form's text field, checkbox, slider
//! and buttons are `ui::Field`, `ui::Input`, `ui::Checkbox`, `ui::Slider` and `ui::Button`, at the board's
//! numbers through their recipes, and what the form says of them (an error, a required mark, a disabled control)
//! and does with them (submit, judge a field the focus leaves, focus) still holds. The legacy theme draws them in
//! the colors 0.62's form theme gave them.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, ThemeDefinition, ThemePalette},
  },
  components::{
    Column, FormCheckboxInput, FormCheckboxInputProps, FormHandle, FormOptions, FormPrimaryButton,
    FormPrimaryButtonProps, FormSecondaryButton, FormSecondaryButtonProps, FormSliderInput, FormSliderInputProps,
    FormTextInput, FormTextInputProps, FormValues, Row, ValidationResult, validators,
  },
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::{Element, color::Color},
  ui::{Button, ButtonVariant, Checkbox, Field, Input, Length, Slider, SliderPart},
};

use super::stage::Stage;

const TEXT: u8 = 1;
const CHECKBOX: u8 = 2;
const SLIDER: u8 = 4;
const BUTTONS: u8 = 8;
/// Beside the form's controls, the design system's own, built as an app would build them.
const BESIDE: u8 = 16;

/// What the scene shows, how its controls are told, and what the test reads back of the form.
#[derive(Clone, Default)]
struct Probe {
  show: u8,
  required: bool,
  disabled: bool,
  /// The text field is many lines, 120 high.
  multiline: bool,
  submitted: Arc<Mutex<Option<FormValues>>>,
  form: Arc<Mutex<Option<FormHandle>>>,
}

impl Probe {
  fn showing(show: u8) -> Self {
    Self {
      show,
      ..Self::default()
    }
  }

  fn form(&self) -> FormHandle {
    self.form.lock().unwrap().clone().expect("the form is made")
  }

  fn submitted(&self) -> Option<FormValues> {
    self.submitted.lock().unwrap().clone()
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Probe {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl PartialEq for Probe {
  fn eq(&self, other: &Self) -> bool {
    (self.show, self.required, self.disabled, self.multiline)
      == (other.show, other.required, other.disabled, other.multiline)
      && Arc::ptr_eq(&self.submitted, &other.submitted)
  }
}

struct Signup {
  form: FormHandle,
  probe: Probe,
}

impl Component for Signup {
  type Props = Probe;

  fn create(ctx: &mut Ctx) -> Self {
    let probe = ctx.props::<Probe>().clone();
    let submitted = probe.submitted.clone();
    let mut options = FormOptions::new()
      .field("email", "")
      .field("terms", false)
      .field("volume", 40.0)
      .validate_string("email", validators::required("Enter your email"));
    // The terms are judged where their box is shown.
    if probe.show & CHECKBOX != 0 {
      options = options.validate_bool("terms", |accepted, _| {
        if accepted {
          ValidationResult::valid()
        } else {
          ValidationResult::invalid("Accept the terms")
        }
      });
    }
    let form = ctx.form(options).on_submit(move |values| {
      *submitted.lock().unwrap() = Some(values);
    });
    *probe.form.lock().unwrap() = Some(form.clone());
    Self { form, probe }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = self.probe.clone();
    let shows = |what: u8| probe.show & what != 0;
    // A control is made only for what is shown: a checkbox's field that the form holds and no box gives is false
    // when the form is submitted.
    let form = self.form.clone();
    ctx.form_view(self.form.clone(), |ctx| {
      let mut column = Column::new().width(180.0).spacing(16.0).overflow_visible();
      if shows(TEXT) {
        let mut text = FormTextInputProps::new(
          form
            .string_control("email")
            .required(probe.required)
            .disabled(probe.disabled),
        )
        .label("Email")
        .hint("We never share it.")
        .placeholder("you@example.com");
        if probe.multiline {
          text = text.multiline().height(120.0);
        }
        column = column.child(
          Column::new()
            .id("email")
            .overflow_visible()
            .child(ctx.mount::<FormTextInput>(text)),
        );
      }
      if shows(CHECKBOX) {
        column = column.child(
          Column::new().id("terms").overflow_visible().child(
            ctx.mount::<FormCheckboxInput>(
              FormCheckboxInputProps::new(form.bool_control("terms").disabled(probe.disabled))
                .label("I accept")
                .hint("Read them first."),
            ),
          ),
        );
      }
      if shows(SLIDER) {
        column = column.child(
          Column::new().id("volume").overflow_visible().child(
            ctx.mount::<FormSliderInput>(
              FormSliderInputProps::new(form.number_control("volume").disabled(probe.disabled))
                .label("Volume")
                .range(0, 100),
            ),
          ),
        );
      }
      if shows(BUTTONS) {
        column = column
          .child(
            Column::new()
              .id("save")
              .overflow_visible()
              .child(ctx.mount::<FormPrimaryButton>(FormPrimaryButtonProps::new("Create account"))),
          )
          .child(
            Column::new()
              .id("cancel")
              .overflow_visible()
              .child(ctx.mount::<FormSecondaryButton>(FormSecondaryButtonProps::new("Cancel"))),
          );
      }
      let mut scene = Row::new().padding(10.0).spacing(20.0).overflow_visible().child(column);
      if shows(BESIDE) {
        scene = scene.child(beside(probe.show));
      }
      scene
    })
  }
}

/// The design system's own field, input, checkbox, slider and buttons, as an app builds them, in a column as wide
/// as the form's: what the form's controls are to look like.
fn beside(show: u8) -> Element {
  let shows = |what: u8| show & what != 0;
  let mut column = Column::new().width(180.0).spacing(16.0).overflow_visible();
  if shows(TEXT) {
    column = column.child(
      Column::new().id("email-ui").overflow_visible().child(
        Field::new(
          "Email",
          Input::new(Signal::new(String::new()))
            .placeholder("you@example.com")
            .full_width(true),
        )
        .description("We never share it.")
        .full_width(true),
      ),
    );
  }
  if shows(CHECKBOX) {
    column = column.child(
      Column::new().id("terms-ui").overflow_visible().child(
        Field::new("", Checkbox::new("I accept").bind(Signal::new(false)))
          .description("Read them first.")
          .full_width(true),
      ),
    );
  }
  if shows(SLIDER) {
    column = column.child(
      Column::new().id("volume-ui").overflow_visible().child(
        Field::new(
          "Volume",
          Slider::new(Signal::new(40)).style(SliderPart::Root, |s| s.width(Length::Fill)),
        )
        .full_width(true),
      ),
    );
  }
  if shows(BUTTONS) {
    column = column
      .child(
        Column::new()
          .id("save-ui")
          .overflow_visible()
          .child(Button::new("Create account").full_width(true)),
      )
      .child(
        Column::new()
          .id("cancel-ui")
          .overflow_visible()
          .child(Button::new("Cancel").variant(ButtonVariant::Outline).full_width(true)),
      );
  }
  column.into()
}

fn stage(probe: &Probe) -> Stage {
  stage_in(App::new(), probe)
}

fn stage_in(app: App, probe: &Probe) -> Stage {
  let probe = probe.clone();
  Stage::in_app(app, move |ctx| ctx.mount::<Signup>(probe.clone()))
}

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The quads painted inside the box of the element with `id`.
fn inside(stage: &mut Stage, id: &str) -> Vec<Quad> {
  let (x, y, width, height) = stage.bounds(id);
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      quad.x >= x - 0.5
        && quad.y >= y - 0.5
        && quad.x + quad.width <= x + width + 0.5
        && quad.y + quad.height <= y + height + 0.5
    })
    .collect()
}

/// What is painted inside the element with `id`, from its corner: where each thing is, how large, and in what
/// colors, one line each.
fn picture(stage: &mut Stage, id: &str) -> Vec<String> {
  let (x, y, ..) = stage.bounds(id);
  inside(stage, id)
    .into_iter()
    .map(|quad| {
      let at = format!(
        "{:.1},{:.1} {:.1}x{:.1}",
        quad.x - x,
        quad.y - y,
        quad.width,
        quad.height
      );
      let edge = quad
        .border
        .and_then(|border| border.top)
        .map(|side| format!(" edge {} {:?}", side.width, side.color))
        .unwrap_or_default();
      match &quad.content {
        QuadContent::Text { text, style, .. } => format!("{at} text {text:?} {:?} {}", style.color, style.font_size),
        QuadContent::Rect { color, .. } => format!("{at} fill {color:?}{edge}"),
        _ => format!("{at} other{edge}"),
      }
    })
    .collect()
}

/// The quad with an edge that is `width` by `height`, inside the element with `id`.
fn boxed(stage: &mut Stage, id: &str, width: f32, height: f32) -> Quad {
  inside(stage, id)
    .into_iter()
    .find(|quad| quad.border.is_some() && (quad.width - width).abs() < 0.5 && (quad.height - height).abs() < 0.5)
    .unwrap_or_else(|| panic!("no box of {width} by {height} in #{id}"))
}

fn edge_of(quad: &Quad) -> (f32, Color) {
  let side = quad.border.and_then(|border| border.top).expect("an edge");
  (side.width, side.color)
}

fn fill_of(quad: &Quad) -> Color {
  match quad.content {
    QuadContent::Rect { color, .. } => color,
    _ => panic!("not a filled box"),
  }
}

/// The filled box `width` by `height` inside the element with `id`, of a color that shows.
fn filled(stage: &mut Stage, id: &str, width: f32, height: f32) -> Quad {
  inside(stage, id)
    .into_iter()
    .find(|quad| {
      matches!(quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && (quad.width - width).abs() < 0.5
        && (quad.height - height).abs() < 0.5
    })
    .unwrap_or_else(|| panic!("no filled box of {width} by {height} in #{id}"))
}

fn click_at(stage: &mut Stage, (x, y): (f32, f32)) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  stage.pass();
}

fn center(quad: &Quad) -> (f32, f32) {
  (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0)
}

/// Types `text` at the start of the focused input.
fn type_in(stage: &mut Stage, text: &str) {
  stage.tree.text_input_replace(0..0, text);
  stage.pass();
}

fn submit(stage: &mut Stage) {
  stage.click("save");
}

/// Fills in the email the form requires, where the scene does not show its field or the test is not about it.
fn fill_in(probe: &Probe) {
  probe.form().string("email").set("ivan@lurq.dev".to_owned());
}

fn tab(stage: &mut Stage) {
  stage.key("Tab");
}

#[test]
fn each_form_control_paints_what_the_design_systems_own_paints() {
  let mut stage = stage(&Probe::showing(TEXT | CHECKBOX | SLIDER | BUTTONS | BESIDE));
  for id in ["email", "terms", "volume", "save", "cancel"] {
    let form = picture(&mut stage, id);
    assert!(!form.is_empty(), "#{id} paints something");
    assert_eq!(form, picture(&mut stage, &format!("{id}-ui")), "#{id}");
  }
}

#[test]
fn the_text_field_is_the_boards_input_under_the_fields_label() {
  let mut stage = stage(&Probe::showing(TEXT));
  // An input 36 high as wide as the field, the background's fill in an edge of 1 in the input's color.
  let input = boxed(&mut stage, "email", 180.0, 36.0);
  assert_eq!(edge_of(&input), (1.0, light(PaletteColor::Input)));
  assert_eq!(fill_of(&input), light(PaletteColor::Background));
  // The label in a field label's type (13) and the foreground, the hint in a caption's (12) and the muted
  // foreground, 6 under the box.
  assert_eq!(stage.text("Email"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(
    stage.text("We never share it."),
    (light(PaletteColor::MutedForeground), 12.0)
  );
  let hint = stage.text_quad("We never share it.").unwrap();
  assert!(
    (hint.y - (input.y + input.height + 6.0)).abs() < 1.0,
    "{} {}",
    hint.y,
    input.y
  );
}

#[test]
fn an_error_of_the_form_takes_the_hints_place_and_an_edit_clears_it() {
  let probe = Probe::showing(TEXT | BUTTONS);
  let mut stage = stage(&probe);
  submit(&mut stage);
  assert!(probe.submitted().is_none(), "an empty email is not submitted");
  assert_eq!(stage.text("Enter your email").0, light(PaletteColor::Destructive));
  assert!(!stage.paints("We never share it."));
  let input = boxed(&mut stage, "email", 180.0, 36.0);
  assert_eq!(
    edge_of(&input).1,
    light(PaletteColor::Destructive),
    "the box is invalid"
  );

  click_at(&mut stage, center(&input));
  type_in(&mut stage, "ivan@lurq.dev");
  assert!(!stage.paints("Enter your email"), "an edit clears the error");
  assert!(stage.paints("We never share it."));
  submit(&mut stage);
  let values = probe.submitted().expect("submitted");
  assert_eq!(values.get_string("email"), Some("ivan@lurq.dev"));
}

#[test]
fn a_required_control_has_the_fields_mark_and_a_press_of_its_label_focuses_it() {
  let mut probe = Probe::showing(TEXT);
  probe.required = true;
  let mut stage = stage(&probe);
  assert_eq!(stage.text("*").0, light(PaletteColor::Destructive));

  let label = stage.text_quad("Email").unwrap();
  click_at(&mut stage, center(&label));
  let focused = stage
    .tree
    .focused_element()
    .map(|element| element.tag_name().to_owned());
  assert_eq!(focused.as_deref(), Some("TextInput"));
}

#[test]
fn the_checkbox_is_the_boards_with_its_label_beside_it() {
  let probe = Probe::showing(CHECKBOX | BUTTONS);
  let mut stage = stage(&probe);
  // A box of 16 with an edge of 1 in the input's color, its label 8 beside it in the body's type, the hint under.
  let square = boxed(&mut stage, "terms", 16.0, 16.0);
  assert_eq!(edge_of(&square), (1.0, light(PaletteColor::Input)));
  let label = stage.text_quad("I accept").unwrap();
  assert!(
    (label.x - (square.x + 16.0 + 8.0)).abs() < 1.0,
    "{} {}",
    label.x,
    square.x
  );
  assert_eq!(stage.text("I accept"), (light(PaletteColor::Foreground), 14.0));
  assert!(stage.paints("Read them first."));
  assert!(!stage.paints("Accept the terms"));

  // Checked, it is filled in the primary color, and the form is given `on`.
  fill_in(&probe);
  click_at(&mut stage, center(&square));
  assert!(probe.form().bool("terms").get_untracked());
  assert_eq!(
    fill_of(&filled(&mut stage, "terms", 16.0, 16.0)),
    light(PaletteColor::Primary)
  );
  submit(&mut stage);
  assert_eq!(probe.submitted().expect("submitted").get_bool("terms"), Some(true));

  // A press of its label is a press of it; one not checked gives the form nothing, which is false.
  click_at(&mut stage, center(&label));
  assert!(!probe.form().bool("terms").get_untracked());
  submit(&mut stage);
  assert!(stage.paints("Accept the terms"), "the form judged it");
  assert_eq!(probe.form().values().get_bool("terms"), Some(false));
}

#[test]
fn the_form_judges_the_checkbox_the_focus_leaves() {
  let mut stage = stage(&Probe::showing(CHECKBOX | BUTTONS));
  tab(&mut stage);
  assert!(!stage.paints("Accept the terms"));
  tab(&mut stage);
  assert_eq!(stage.text("Accept the terms").0, light(PaletteColor::Destructive));
  let square = boxed(&mut stage, "terms", 16.0, 16.0);
  assert_eq!(
    edge_of(&square).1,
    light(PaletteColor::Destructive),
    "the box is invalid"
  );
}

#[test]
fn the_slider_is_the_boards_as_wide_as_its_field() {
  let probe = Probe::showing(SLIDER | BUTTONS);
  let mut stage = stage(&probe);
  let slider = stage
    .tree
    .find_element(|element| element.tag_name() == "Slider")
    .expect("the engine's slider")
    .bounds();
  assert_eq!((slider.width, slider.height), (180.0, 16.0));
  assert_eq!(stage.text("Volume").1, 13.0);

  // The keys move it a step, and the form holds and submits the number.
  tab(&mut stage);
  stage.key("ArrowRight");
  assert_eq!(probe.form().number("volume").get_untracked(), 41.0);
  fill_in(&probe);
  submit(&mut stage);
  assert_eq!(probe.submitted().expect("submitted").get_number("volume"), Some(41.0));
}

#[test]
fn the_buttons_are_the_boards_primary_and_outline_as_wide_as_the_form() {
  let probe = Probe::showing(BUTTONS);
  let mut stage = stage(&probe);
  let save = filled(&mut stage, "save", 180.0, 36.0);
  assert_eq!(fill_of(&save), light(PaletteColor::Primary));
  assert_eq!(
    stage.text("Create account"),
    (light(PaletteColor::PrimaryForeground), 13.0)
  );
  let cancel = boxed(&mut stage, "cancel", 180.0, 36.0);
  assert_eq!(edge_of(&cancel), (1.0, light(PaletteColor::Input)));
  assert_eq!(
    fill_of(&filled(&mut stage, "cancel", 180.0, 36.0)),
    light(PaletteColor::Background)
  );

  fill_in(&probe);
  stage.click("cancel");
  assert!(probe.submitted().is_none(), "the secondary button submits nothing");
  stage.click("save");
  assert!(probe.submitted().is_some(), "the primary one submits");
}

#[test]
fn tab_goes_through_the_controls_in_order_with_the_rings_of_the_design() {
  let mut stage = stage(&Probe::showing(TEXT | CHECKBOX | SLIDER | BUTTONS));
  let ring = light(PaletteColor::Ring);
  assert!(!stage.has_ring(ring));
  for id in ["email", "terms", "volume", "save", "cancel"] {
    tab(&mut stage);
    // The one ring painted is around (or, of the text box, inside) the control the focus went to.
    let rings = stage
      .quads()
      .into_iter()
      .filter(|quad| {
        matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() == 0)
          && quad.border.and_then(|border| border.top).map(|side| side.color) == Some(ring)
      })
      .collect::<Vec<_>>();
    assert_eq!(rings.len(), 1, "one ring at #{id}");
    let (cx, cy) = center(&rings[0]);
    let (x, y, width, height) = stage.bounds(id);
    assert!(
      cx >= x && cx <= x + width && cy >= y && cy <= y + height,
      "the ring is in #{id}"
    );
  }
}

#[test]
fn a_disabled_control_takes_nothing() {
  let mut probe = Probe::showing(TEXT | CHECKBOX);
  probe.disabled = true;
  let mut stage = stage(&probe);
  let input = boxed(&mut stage, "email", 180.0, 36.0);
  assert_eq!(fill_of(&input), light(PaletteColor::Muted));
  click_at(&mut stage, center(&input));
  assert!(stage.tree.focused_element().is_none(), "a disabled box takes no focus");

  let square = boxed(&mut stage, "terms", 16.0, 16.0);
  click_at(&mut stage, center(&square));
  assert!(!probe.form().bool("terms").get_untracked());
}

/// The colors of 0.62's form theme, which the legacy palette's roles keep.
#[test]
fn the_legacy_theme_draws_them_in_the_colors_of_0_62() {
  let app = App::new();
  app.theme().apply(ThemeDefinition::legacy());
  let probe = Probe::showing(TEXT | CHECKBOX | BUTTONS);
  let mut stage = stage_in(app, &probe);
  let (accent, inverse, danger) = (
    Color::from_hex("#2563eb"),
    Color::from_hex("#ffffff"),
    Color::from_hex("#dc2626"),
  );
  // The primary button in the accent with the inverse text on it.
  assert_eq!(fill_of(&filled(&mut stage, "save", 180.0, 36.0)), accent);
  assert_eq!(stage.text("Create account").0, inverse);
  // A checked box in the accent.
  let square = boxed(&mut stage, "terms", 16.0, 16.0);
  click_at(&mut stage, center(&square));
  assert_eq!(fill_of(&filled(&mut stage, "terms", 16.0, 16.0)), accent);
  // The error in the danger color, and the focus in the focus border's color.
  submit(&mut stage);
  assert_eq!(stage.text("Enter your email").0, danger);
  let input = boxed(&mut stage, "email", 180.0, 36.0);
  click_at(&mut stage, center(&input));
  assert!(stage.has_ring(accent));
}

#[test]
fn a_multiline_text_field_is_the_boards_textarea_at_the_height_it_is_given() {
  let mut probe = Probe::showing(TEXT | BUTTONS);
  probe.multiline = true;
  let mut stage = stage(&probe);
  let area = boxed(&mut stage, "email", 180.0, 120.0);
  assert_eq!(edge_of(&area), (1.0, light(PaletteColor::Input)));
  // Enter is a new line in it, and submits nothing.
  click_at(&mut stage, center(&area));
  type_in(&mut stage, "ivan@lurq.dev");
  stage.key("Enter");
  assert!(probe.submitted().is_none());
  submit(&mut stage);
  assert!(probe.submitted().is_some());
}
