//! Components in a room narrower than the width their recipe gives them, as a window as narrow as a phone
//! leaves them: each is as wide as the room, and nothing it says is laid out past the room. The width of a
//! recipe is the most a component takes, not the least.

use lurq::{
  app::Accelerator,
  components::{Column, Row},
  core::{Date, DateRange, Signal, Time},
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{
    AudioPlayer, AudioPlayerVariant, CheckboxCard, ColorField, Combobox, DateField, DatePicker, DateRangePicker,
    DropZone, ErrorSummary, Field, FieldError, Fieldset, FileUpload, Input, InputGroup, Mentions, MentionsController,
    MessageBox, MessageBoxRole, MultiSelect, NumberInput, Outcome, PasswordInput, Progress, ProgressDialog, Property,
    PropertyCategory, PropertyGrid, SearchField, Select, ShortcutRecorder, Textarea, TimeField, TimePicker, Toast,
    TokenInput, Waveform,
  },
};

use super::stage::Stage;

/// How wide the room is: less than any of the recipes' widths.
const ROOM: f32 = 200.0;

/// A choice of a select: its value and what it shows.
fn themes() -> Vec<(String, &'static str)> {
  vec![
    ("system".to_owned(), "System"),
    ("light".to_owned(), "Light"),
    ("dark".to_owned(), "Dark"),
  ]
}

/// Lays `component`, whose id is `it`, out in a room of [`ROOM`] and says that it is no wider than the room and
/// that no text of it is past the room's end.
#[track_caller]
fn fits(component: impl Fn() -> Element + Send + Sync + 'static) {
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      // In a row, which lays what it holds out as wide as it makes itself: nothing but the component's own
      // greatest width keeps it in the room.
      .child(Row::new().id("room").width(ROOM).child(component()))
      .into()
  });
  let (room, _, room_width, _) = stage.bounds("room");
  let (x, _, width, _) = stage.bounds("it");
  assert!(
    x >= room - 0.5 && x + width <= room + room_width + 0.5,
    "it is {width} wide at {x}, in a room of {room_width} at {room}"
  );
  for quad in stage.quads() {
    if let QuadContent::Text { text, .. } = &quad.content {
      assert!(
        quad.x + quad.width <= room + room_width + 0.5,
        "{text:?} is {} wide at {}, past the end of the room at {}",
        quad.width,
        quad.x,
        room + room_width
      );
    }
  }
}

fn text() -> Signal<String> {
  Signal::new("Ivan Zaida".to_owned())
}

#[test]
fn an_input_is_no_wider_than_a_narrow_room() {
  fits(|| Input::new(text()).id("it").into());
}

#[test]
fn a_textarea_is_no_wider_than_a_narrow_room() {
  fits(|| Textarea::new(text()).id("it").into());
}

#[test]
fn a_password_input_is_no_wider_than_a_narrow_room() {
  fits(|| PasswordInput::new(text()).id("it").into());
}

#[test]
fn a_search_field_is_no_wider_than_a_narrow_room() {
  fits(|| SearchField::new(text()).id("it").into());
}

#[test]
fn an_input_group_is_no_wider_than_a_narrow_room() {
  fits(|| InputGroup::new(Input::new(text())).id("it").into());
}

#[test]
fn a_token_input_is_no_wider_than_a_narrow_room() {
  fits(|| TokenInput::new(text()).id("it").into());
}

#[test]
fn mentions_are_no_wider_than_a_narrow_room() {
  fits(|| Mentions::new(text(), &MentionsController::new()).id("it").into());
}

#[test]
fn a_select_is_no_wider_than_a_narrow_room() {
  fits(|| Select::new(Signal::new("light".to_owned()), themes()).id("it").into());
}

#[test]
fn a_combobox_is_no_wider_than_a_narrow_room() {
  fits(|| Combobox::new(Signal::new("light".to_owned()), themes()).id("it").into());
}

#[test]
fn a_multi_select_is_no_wider_than_a_narrow_room() {
  fits(|| {
    MultiSelect::new(Signal::new(vec!["light".to_owned()]), themes())
      .id("it")
      .into()
  });
}

#[test]
fn a_field_is_no_wider_than_a_narrow_room() {
  fits(|| {
    Field::new("Display name", Input::new(text()))
      .description("Shown beside what you write, to everyone who can read it.")
      .id("it")
      .into()
  });
}

#[test]
fn a_fieldset_is_no_wider_than_a_narrow_room() {
  fits(|| {
    Fieldset::new("Profile")
      .description("What others see of you, wherever your name is shown.")
      .child(Field::new("Display name", Input::new(text())))
      .id("it")
      .into()
  });
}

#[test]
fn an_error_summary_is_no_wider_than_a_narrow_room() {
  fits(|| {
    ErrorSummary::new([FieldError::new("Email", "Email must include a domain")])
      .id("it")
      .into()
  });
}

#[test]
fn a_file_upload_is_no_wider_than_a_narrow_room() {
  fits(|| FileUpload::new(Signal::new(Vec::new())).id("it").into());
}

#[test]
fn a_drop_zone_is_no_wider_than_a_narrow_room() {
  fits(|| DropZone::new().id("it").into());
}

#[test]
fn a_progress_is_no_wider_than_a_narrow_room() {
  fits(|| Progress::new(0.6).label("Uploading").id("it").into());
}

#[test]
fn a_result_is_no_wider_than_a_narrow_room() {
  fits(|| {
    Outcome::new("Build succeeded")
      .description("Every check passed, and the build is ready to be deployed.")
      .id("it")
      .into()
  });
}

#[test]
fn a_toast_is_no_wider_than_a_narrow_room() {
  fits(|| {
    Toast::new("Settings saved")
      .description("Your changes are on every device you are signed in to.")
      .id("it")
      .into()
  });
}

#[test]
fn a_message_box_in_the_flow_is_no_wider_than_a_narrow_room() {
  fits(|| {
    MessageBox::new(Signal::new(true))
      .inline(true)
      .title("Replace existing file?")
      .description("theme.json already exists in this folder.")
      .answer("Cancel", MessageBoxRole::Cancel, || ())
      .answer("Replace", MessageBoxRole::Default, || ())
      .id("it")
      .into()
  });
}

#[test]
fn a_progress_dialog_in_the_flow_is_no_wider_than_a_narrow_room() {
  fits(|| {
    ProgressDialog::new(Signal::new(true))
      .inline(true)
      .title("Copying 128 items")
      .description("from Downloads to projects/lurq/assets")
      .value(32.0)
      .max(128.0)
      .id("it")
      .into()
  });
}

#[test]
fn a_property_grid_is_no_wider_than_a_narrow_room() {
  fits(|| {
    PropertyGrid::new()
      .title("Properties")
      .category(PropertyCategory::new("Appearance").property(Property::text("Name", text())))
      .id("it")
      .into()
  });
}

#[test]
fn a_field_with_a_control_of_its_own_width_is_no_wider_than_a_narrow_room() {
  fits(|| {
    Field::new("Seats", NumberInput::new(Signal::new(4.0)))
      .description("From 1 to 12 at one table, and no more than the room has.")
      .id("it")
      .into()
  });
}

#[test]
fn a_choice_card_is_no_wider_than_a_narrow_room() {
  fits(|| {
    CheckboxCard::new("Crash reports")
      .description("Send a report when the app closes without being asked to.")
      .id("it")
      .into()
  });
}

fn day() -> Date {
  Date::new(2026, 11, 12).unwrap()
}

/// A room with `picker` in it, which has no id of its own box: the room holds what it says.
#[track_caller]
fn holds(picker: impl Fn() -> Element + Send + Sync + 'static) {
  fits(move || {
    Row::new()
      .id("it")
      .max_width(lurq::node::dimension::Dimension::Pct(100.0))
      .child(picker())
      .into()
  });
}

#[test]
fn a_date_picker_is_no_wider_than_a_narrow_room() {
  holds(|| DatePicker::new(Signal::new(Some(day()))).into());
}

#[test]
fn a_date_range_picker_is_no_wider_than_a_narrow_room() {
  holds(|| DateRangePicker::new(Signal::new(Some(DateRange::new(day(), day())))).into());
}

#[test]
fn a_time_picker_is_no_wider_than_a_narrow_room() {
  holds(|| TimePicker::new(Signal::new(Time::new(9, 30, 0).unwrap())).into());
}

#[test]
fn a_color_field_is_no_wider_than_a_narrow_room() {
  holds(|| ColorField::new(Signal::new(Color::from_hex("#2563eb"))).into());
}

#[test]
fn a_date_field_is_no_wider_than_a_narrow_room() {
  holds(|| DateField::new(Signal::new(day())).into());
}

#[test]
fn a_time_field_is_no_wider_than_a_narrow_room() {
  holds(|| TimeField::new(Signal::new(Time::new(14, 2, 0).unwrap())).into());
}

#[test]
fn a_shortcut_recorder_is_no_wider_than_a_narrow_room() {
  holds(|| ShortcutRecorder::optional(Signal::new(None::<Accelerator>)).into());
}

#[test]
fn the_answers_of_a_narrow_message_box_go_under_its_checkbox() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new().id("room").width(260.0).child(
          MessageBox::new(Signal::new(true))
            .inline(true)
            .title("Replace existing file?")
            .description("theme.json already exists in this folder.")
            .checkbox("Don't ask again", Signal::new(false))
            .answer("Cancel", MessageBoxRole::Cancel, || ())
            .answer("Replace", MessageBoxRole::Default, || ())
            .id("it"),
        ),
      )
      .into()
  });
  let (room, _, room_width, _) = stage.bounds("room");
  let end = room + room_width;
  let mut answers = 0;
  let mut option = None;
  for quad in stage.quads() {
    if let QuadContent::Text { text, .. } = &quad.content {
      assert!(
        quad.x + quad.width <= end + 0.5,
        "{text:?} ends at {}, past the room's end at {end}",
        quad.x + quad.width
      );
      match text.as_str() {
        "Don't ask again" => option = Some(quad.y),
        "Cancel" | "Replace" => {
          answers += 1;
          assert!(
            option.is_some_and(|option| quad.y > option + 8.0),
            "{text:?} is on a line under the checkbox"
          );
        }
        _ => {}
      }
    }
  }
  assert_eq!(answers, 2, "both answers are drawn");
}

#[test]
fn a_toast_in_a_narrow_room_has_all_of_its_timer_in_it() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new()
          .id("room")
          .width(ROOM)
          .child(Toast::new("Settings saved").timer(true).id("it")),
      )
      .into()
  });
  let (x, _, width, _) = stage.bounds("it");
  // The rail of the timer and what is left of it are the two lines of 2 at the foot of the toast: each ends where
  // the toast does, and what is left starts as long as the rail.
  let lines: Vec<_> = stage
    .quads()
    .into_iter()
    .filter(|quad| (quad.height - 2.0).abs() < 0.01 && matches!(quad.content, QuadContent::Rect { .. }))
    .collect();
  assert!(
    lines.len() >= 2,
    "the rail and what is left of the time are drawn: {}",
    lines.len()
  );
  for line in lines {
    assert!(
      line.x + line.width <= x + width + 0.5,
      "a line of the timer ends at {}, past the toast's end at {}",
      line.x + line.width,
      x + width
    );
  }
}

/// A field with its label beside a switch, in a room `width` wide which the signal changes.
fn beside(width: &Signal<f32>) -> Stage {
  let width = width.clone();
  let mut stage = Stage::first_frame(600.0, 300.0, move |_| {
    Column::new()
      .size(600.0, 300.0)
      .child(
        Row::new().id("room").width(width.get()).child(
          Field::new(
            "Notifications",
            lurq::ui::Switch::new("Pause notifications").id("switch"),
          )
          .description("Nothing is sent while they are paused.")
          .orientation(lurq::ui::Orientation::Horizontal)
          .id("it"),
        ),
      )
      .into()
  });
  stage.pass();
  stage
}

/// Where the label and the switch of [`beside`] are: the label's top and bottom, the switch's start and top.
fn label_and_switch(stage: &mut Stage) -> ((f32, f32), (f32, f32)) {
  let label = stage.text_quad("Notifications").expect("the label is drawn");
  let (x, y, ..) = stage.bounds("switch");
  ((label.y, label.y + label.height), (x, y))
}

#[test]
fn a_field_with_its_label_beside_has_the_label_over_the_control_in_a_narrow_room() {
  let width = Signal::new(256.0);
  let mut stage = beside(&width);
  let (room, _, room_width, _) = stage.bounds("room");
  let (x, _, field, _) = stage.bounds("it");
  assert!(
    x >= room - 0.5 && x + field <= room + room_width + 0.5,
    "the field is {field} wide at {x}, in a room of {room_width}"
  );
  for quad in stage.quads() {
    if let QuadContent::Text { text, .. } = &quad.content {
      assert!(
        quad.x + quad.width <= room + room_width + 0.5,
        "{text:?} ends at {}, past the room's end",
        quad.x + quad.width
      );
    }
  }
  let ((_, label_bottom), (switch_x, switch_y)) = label_and_switch(&mut stage);
  assert!(
    switch_y >= label_bottom - 0.5,
    "the switch at {switch_y} is under the label, which ends at {label_bottom}"
  );
  assert!(
    (switch_x - x).abs() < 0.5,
    "and starts where the field does: {switch_x}"
  );
  // The label is as high as its words there, not as a control: it is not beside one.
  let label = stage.text_quad("Notifications").unwrap();
  assert!(
    switch_y - (label.y + label.height) < 24.0,
    "the switch is close under the label: {} apart",
    switch_y - (label.y + label.height)
  );
}

#[test]
fn a_field_with_its_label_beside_keeps_it_beside_where_there_is_room_and_goes_back_to_it() {
  let width = Signal::new(400.0);
  let mut stage = beside(&width);
  let wide = label_and_switch(&mut stage);
  let ((label_top, label_bottom), (switch_x, switch_y)) = wide;
  let (x, _, field, _) = stage.bounds("it");
  assert_eq!(field, 360.0, "the field is the recipe's 360");
  assert!(
    (switch_x - (x + 122.0)).abs() < 0.5,
    "the switch after the label's 110 and 12: {}",
    switch_x - x
  );
  assert!(
    switch_y < label_bottom && switch_y + 10.0 > label_top,
    "the switch is level with the label: {switch_y} beside {label_top} to {label_bottom}"
  );
  // Narrow, the label goes over the switch; wide again, it is beside it as it was.
  width.set(256.0);
  stage.pass();
  stage.pass();
  let ((_, label_bottom), (_, switch_y)) = label_and_switch(&mut stage);
  assert!(switch_y >= label_bottom - 0.5, "narrow, the switch is under the label");
  width.set(400.0);
  stage.pass();
  stage.pass();
  assert_eq!(label_and_switch(&mut stage), wide, "wide again, as it was");
}

#[test]
fn the_first_frame_of_a_field_in_a_narrow_room_is_the_settled_one() {
  let scene = |_: &mut lurq::app::ctx::Ctx| -> Element {
    Column::new()
      .size(600.0, 300.0)
      .child(
        Row::new().id("room").width(256.0).child(
          Field::new(
            "Notifications",
            lurq::ui::Switch::new("Pause notifications").id("switch"),
          )
          .orientation(lurq::ui::Orientation::Horizontal)
          .id("it"),
        ),
      )
      .into()
  };
  let mut stage = Stage::first_frame(600.0, 300.0, scene);
  let first = stage.picture();
  stage.frame();
  assert_eq!(first, stage.picture(), "nothing moves in the second frame");
}

#[test]
fn a_field_with_its_label_beside_an_input_that_fills_folds_in_a_narrow_room_too() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new().id("room").width(256.0).child(
          Field::new("Time zone", Input::new(text()).full_width(true).id("input"))
            .orientation(lurq::ui::Orientation::Horizontal)
            .id("it"),
        ),
      )
      .into()
  });
  let label = stage.text_quad("Time zone").expect("the label is drawn");
  let (x, y, width, _) = stage.bounds("input");
  let (field_x, _, field, _) = stage.bounds("it");
  assert!(y >= label.y + label.height - 0.5, "the input at {y} is under the label");
  assert!(
    (x - field_x).abs() < 0.5 && (width - field).abs() < 0.5 && (field - 256.0).abs() < 0.5,
    "and as wide as the field, which is as wide as the room: {width} at {x} in {field}"
  );
}

#[test]
fn an_audio_player_is_no_wider_than_a_narrow_room() {
  for variant in [
    AudioPlayerVariant::Full,
    AudioPlayerVariant::Compact,
    AudioPlayerVariant::Voice,
  ] {
    fits(move || {
      AudioPlayer::controlled(84.0, 250.0, |_| {})
        .variant(variant)
        .title("Night Drive")
        .description("Aurora Lane")
        .peaks(&[0.5; 36])
        .id("it")
        .build(&lurq::ui::current_theme())
    });
  }
}

#[test]
fn a_waveform_in_a_narrow_room_keeps_its_bars_and_closes_its_gaps() {
  // 60 bars of 3 with gaps of 2 are 298 wide: more than the room.
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new()
          .id("room")
          .width(ROOM)
          .child(Waveform::at(0.0, &[0.5; 60]).unheard(true).id("it")),
      )
      .into()
  });
  let (room, _, room_width, _) = stage.bounds("room");
  let (x, _, width, _) = stage.bounds("it");
  assert!(
    x >= room - 0.5 && x + width <= room + room_width + 0.5,
    "it is {width} wide at {x}"
  );
  let bars: Vec<_> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(quad.content, QuadContent::Rect { .. }) && (quad.width - 3.0).abs() < 0.01)
    .collect();
  assert_eq!(bars.len(), 60, "every bar is drawn, 3 wide");
  assert!((bars[0].x - room).abs() < 0.5, "the first at the start of the room");
  let last = bars.last().unwrap();
  assert!(
    (last.x + last.width - (room + room_width)).abs() < 0.5,
    "the last at its end"
  );
}
