//! The rest of the button family: a toggle, the groups, a segmented control,
//! a link, a floating action button, and a button with a description.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::{Column, Row},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{
    Button, ButtonGroup, ButtonGroupVariant, ButtonVariant, Choice, Fab, Link, SegmentedControl, Toggle, ToggleGroup,
    icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The character lurq's font has for an icon.
fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find_map(|(known, glyph)| (*known == name).then(|| glyph.to_string()))
    .unwrap_or_else(|| panic!("lurq's font has no icon {name:?}"))
}

/// The filled rectangles painted exactly in the box of the element with `id`.
fn fills_in(stage: &mut Stage, id: &str) -> Vec<Quad> {
  let (x, y, width, height) = stage.bounds(id);
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && quad.x >= x - 0.01
        && quad.y >= y - 0.01
        && quad.x + quad.width <= x + width + 0.01
        && quad.y + quad.height <= y + height + 0.01
    })
    .collect()
}

fn color_of(quad: &Quad) -> Color {
  match quad.content {
    QuadContent::Rect { color, .. } => color,
    _ => panic!("a filled rectangle"),
  }
}

#[test]
fn a_toggle_is_filled_while_it_is_on_and_a_press_asks_for_the_other_state() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let mut stage = Stage::new(move |_| {
    let (off, on) = (told.clone(), told.clone());
    Row::new()
      .spacing(8.0)
      .child(
        Toggle::new("Bold")
          .icon(icons::BOLD)
          .id("off")
          .on_change(move |next| off.lock().unwrap().push(("off", next))),
      )
      .child(
        Toggle::new("Italic")
          .on(true)
          .id("on")
          .on_change(move |next| on.lock().unwrap().push(("on", next))),
      )
      .child(
        Toggle::new("Underline")
          .icon(icons::UNDERLINE)
          .icon_only(true)
          .id("icon"),
      )
      .child(Toggle::new("Gone").disabled(true).id("disabled"))
      .into()
  });
  assert_eq!(stage.bounds("off").3, 36.0, "as high as a button");
  assert_eq!(stage.fill_color("off"), None, "off: no fill of its own");
  assert_eq!(stage.fill_color("on"), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.text("Italic").0, light(PaletteColor::HighlightForeground));
  let (x, ..) = stage.bounds("off");
  let icon = stage.text_quad(&glyph(icons::BOLD)).unwrap();
  assert_eq!(icon.x - x, 10.0, "padded 10");
  let (_, _, width, height) = stage.bounds("icon");
  assert_eq!((width, height), (36.0, 36.0), "an icon alone is a square");
  assert!(!stage.paints("Underline"));

  stage.click("off");
  stage.click("on");
  stage.click("disabled");
  assert_eq!(*asked.lock().unwrap(), vec![("off", true), ("on", false)]);
}

#[test]
fn a_group_joins_its_buttons_in_one_line_around_them() {
  let presses = Arc::new(AtomicUsize::new(0));
  let counted = presses.clone();
  let mut stage = Stage::new(move |_| {
    let counted = counted.clone();
    Column::new()
      .spacing(8.0)
      .child(
        ButtonGroup::new()
          .id("period")
          .child(Button::new("Years").id("years"))
          .child(Button::new("Months").id("months").on_click(move |_| {
            counted.fetch_add(1, Ordering::SeqCst);
          }))
          .child(Button::new("Days").id("days")),
      )
      .child(
        ButtonGroup::new()
          .variant(ButtonGroupVariant::Primary)
          .id("merge")
          .child(Button::new("Merge").id("action"))
          .child(Button::new("How").icon(icons::CHEVRON_DOWN).icon_only(true).id("how")),
      )
      .into()
  });
  // A line around the three, and no line or corner of their own.
  let (x, y, width, height) = stage.bounds("period");
  assert_eq!(height, 36.0);
  let shell = stage.border("period").expect("the group has a line around it");
  let edge = shell.border.and_then(|border| border.top).unwrap();
  assert_eq!((edge.width, edge.color), (1.0, light(PaletteColor::Input)));
  assert_eq!(stage.fill_color("period"), Some(light(PaletteColor::Background)));
  for id in ["years", "months", "days"] {
    assert_eq!(stage.bounds(id).3, 36.0);
    assert_eq!(stage.fill_color(id), None, "{id} has no fill of its own");
    assert!(stage.border(id).is_none(), "{id} has no line of its own");
  }
  // The buttons are side by side with a line of one pixel between them, and fill the group.
  let (years, months, days) = (stage.bounds("years"), stage.bounds("months"), stage.bounds("days"));
  assert_eq!(years.0, x);
  assert_eq!(months.0 - (years.0 + years.2), 1.0, "a separator");
  assert_eq!(days.0 - (months.0 + months.2), 1.0);
  assert_eq!(days.0 + days.2, x + width);
  let lines: Vec<_> = fills_in(&mut stage, "period")
    .into_iter()
    .filter(|quad| quad.width == 1.0 && quad.height == 36.0)
    .collect();
  assert_eq!(lines.len(), 2);
  assert!(lines.iter().all(|line| color_of(line) == light(PaletteColor::Input)));
  let _ = y;
  // A button of the group is padded 14, whatever the theme pads a button.
  let label = stage.text_quad("Years").unwrap();
  assert_eq!((label.x - years.0).round(), 14.0);

  stage.click("months");
  assert_eq!(
    presses.load(Ordering::SeqCst),
    1,
    "a button of a group is pressed as any other"
  );

  // A filled group: its buttons in its fill, a pixel apart.
  let (action, how) = (stage.bounds("action"), stage.bounds("how"));
  assert_eq!(stage.fill_color("action"), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color("how"), Some(light(PaletteColor::Primary)));
  assert_eq!(how.0 - (action.0 + action.2), 1.0);
  assert!(stage.border("merge").is_none());
}

#[test]
fn one_toggle_of_a_group_is_on_or_any_number_are() {
  let align = Signal::new(1u8);
  let marks = Signal::new(vec!["bold"]);
  let (chosen, kept) = (align.clone(), marks.clone());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .spacing(8.0)
      .child(ToggleGroup::new(chosen.clone(), [(0u8, "Left"), (1, "Center"), (2, "Right")]).id("align"))
      .child(
        ToggleGroup::multiple(
          kept.clone(),
          [
            Choice::new("bold", "Bold").icon(icons::BOLD).icon_only(true),
            Choice::new("italic", "Italic").icon(icons::ITALIC).icon_only(true),
          ],
        )
        .id("marks"),
      )
      .into()
  });
  let on = |stage: &mut Stage, label: &str| {
    let text = stage.text_quad(label).unwrap();
    stage.quads().into_iter().any(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Highlight))
        && quad.x <= text.x
        && quad.x + quad.width >= text.x + text.width
        && quad.height == 36.0
        && quad.y <= text.y
        && quad.y + quad.height >= text.y + text.height
    })
  };
  assert!(on(&mut stage, "Center"), "the one whose value the signal has");
  assert!(!on(&mut stage, "Left"));
  assert_eq!(stage.bounds("align").3, 36.0);
  assert!(stage.border("align").is_some(), "a line around the group");

  // A press of another sets the signal, and the group is drawn again.
  let right = stage.text_quad("Right").unwrap();
  stage.tree.mouse_move(right.x + 2.0, right.y + 2.0);
  stage
    .tree
    .mouse_down(right.x + 2.0, right.y + 2.0, lurq::app::events::MouseButton::Left);
  stage
    .tree
    .mouse_up(right.x + 2.0, right.y + 2.0, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert_eq!(align.get(), 2);
  stage.tree.mouse_move(390.0, 290.0);
  stage.pass();
  assert!(on(&mut stage, "Right"));
  assert!(!on(&mut stage, "Center"));

  // Any number: a press adds a value, and a press of one that is on takes it out.
  let press = |stage: &mut Stage, name: &str| {
    let icon = stage.text_quad(&glyph(name)).unwrap();
    let (x, y) = (icon.x + 2.0, icon.y + 2.0);
    stage.tree.mouse_move(x, y);
    stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
    stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
    stage.pass();
  };
  press(&mut stage, icons::ITALIC);
  assert_eq!(marks.get(), vec!["bold", "italic"]);
  press(&mut stage, icons::BOLD);
  assert_eq!(marks.get(), vec!["italic"]);
}

#[test]
fn a_segmented_control_has_one_segment_chosen_on_its_track() {
  let period = Signal::new("week");
  let chosen = period.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .width(300.0)
      .spacing(8.0)
      .child(SegmentedControl::new(chosen.clone(), [("day", "Day"), ("week", "Week"), ("month", "Month")]).id("period"))
      .child(
        SegmentedControl::controlled(1u8, [(0u8, "One"), (1, "Two")], |_| {})
          .full_width(true)
          .id("wide"),
      )
      .into()
  });
  let (x, y, _, height) = stage.bounds("period");
  assert_eq!(height, 36.0);
  assert_eq!(
    stage.fill_color("period"),
    Some(light(PaletteColor::Muted)),
    "the track"
  );
  // The chosen segment is a surface on the track, 3 in from its edge and 30 high.
  let surface = |stage: &mut Stage, id: &str| {
    fills_in(stage, id)
      .into_iter()
      .find(|quad| color_of(quad) == light(PaletteColor::Background))
      .expect("the chosen segment is a surface")
  };
  let week = surface(&mut stage, "period");
  assert_eq!((week.y - y, week.height), (3.0, 30.0));
  let label = stage.text_quad("Week").unwrap();
  assert!(week.x <= label.x && week.x + week.width >= label.x + label.width);
  assert_eq!((label.x - week.x).round(), 14.0, "a segment is padded 14");
  assert_eq!(stage.text("Week"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(stage.text("Day").0, light(PaletteColor::MutedForeground));
  let day = stage.text_quad("Day").unwrap();
  assert_eq!(
    (day.x - x).round(),
    3.0 + 14.0,
    "the first segment is 3 in from the track's edge"
  );

  // A press of another segment sets the signal.
  stage.tree.mouse_move(day.x + 2.0, day.y + 2.0);
  stage
    .tree
    .mouse_down(day.x + 2.0, day.y + 2.0, lurq::app::events::MouseButton::Left);
  stage
    .tree
    .mouse_up(day.x + 2.0, day.y + 2.0, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert_eq!(period.get(), "day");
  let chosen_now = surface(&mut stage, "period");
  let day = stage.text_quad("Day").unwrap();
  assert!(chosen_now.x <= day.x && chosen_now.x + chosen_now.width >= day.x + day.width);

  // As wide as what it is in, its segments sharing the width.
  let (wide_x, _, wide, _) = stage.bounds("wide");
  assert_eq!(wide, 300.0);
  let two = surface(&mut stage, "wide");
  assert_eq!(two.width, (300.0 - 3.0 * 2.0 - 2.0) / 2.0);
  assert_eq!(two.x + two.width, wide_x + 300.0 - 3.0);
}

#[test]
fn a_link_is_text_with_an_icon_after_it_and_a_line_under_the_pointer() {
  let presses = Arc::new(AtomicUsize::new(0));
  let counted = presses.clone();
  let mut stage = Stage::new(move |_| {
    let counted = counted.clone();
    Row::new()
      .padding(20.0)
      .child(Link::new("Read the docs").external().id("docs").on_click(move |_| {
        counted.fetch_add(1, Ordering::SeqCst);
      }))
      .into()
  });
  let (ink, size) = stage.text("Read the docs");
  assert_eq!((ink, size), (light(PaletteColor::Foreground), 14.0));
  let (label, icon) = (
    stage.text_quad("Read the docs").unwrap(),
    stage.text_quad(&glyph(icons::ARROW_UP_RIGHT)).unwrap(),
  );
  assert_eq!(icon.width, 14.0);
  assert_eq!(icon.x - (label.x + label.width), 4.0, "the icon is 4 after the text");
  assert_eq!(stage.fill_color("docs"), None, "a link has no box");
  assert!(stage.border("docs").is_none(), "and no line at rest");

  stage.hover("docs");
  let line = stage
    .border("docs")
    .and_then(|quad| quad.border)
    .expect("a line under the link the pointer is on");
  assert!(line.top.is_none(), "under it only");
  let under = line.bottom.expect("the line");
  assert_eq!((under.width, under.color), (1.0, light(PaletteColor::Foreground)));

  stage.click("docs");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(
    presses.load(Ordering::SeqCst),
    2,
    "a click, and Enter while it is focused"
  );
}

#[test]
fn a_floating_action_button_is_a_raised_circle_with_its_icon() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(20.0)
      .spacing(20.0)
      .child(Fab::new("Compose", icons::COMPOSE).id("fab"))
      .child(Fab::new("Create", icons::PLUS).extended(true).id("wide"))
      .into()
  });
  let (_, _, width, height) = stage.bounds("fab");
  assert_eq!((width, height), (48.0, 48.0));
  let fill = stage.fill("fab").unwrap();
  assert_eq!(color_of(&fill), light(PaletteColor::Primary));
  assert_eq!(
    fill.border_radius.map(|radius| radius.top_left),
    Some(999.0),
    "a circle"
  );
  assert_eq!(stage.text(&glyph(icons::COMPOSE)).1, 20.0, "its icon");
  assert!(
    !stage.paints("Compose"),
    "its label is what it is called, and is not drawn"
  );
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| matches!(quad.content, QuadContent::BoxShadow(_))),
    "it is raised: a shadow under it"
  );

  let (_, _, wide, high) = stage.bounds("wide");
  assert_eq!(high, 48.0);
  assert!(stage.paints("Create"), "extended, it shows its label");
  let (icon, label) = (
    stage.text_quad(&glyph(icons::PLUS)).unwrap(),
    stage.text_quad("Create").unwrap(),
  );
  assert_eq!(label.x - (icon.x + icon.width), 8.0);
  assert_eq!(wide.round(), (20.0 + 20.0 + 8.0 + label.width + 20.0).round());
}

#[test]
fn a_button_with_a_description_is_as_high_as_its_two_lines() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(20.0)
      .child(
        Button::new("Schedule")
          .variant(ButtonVariant::Outline)
          .icon(icons::CALENDAR_PLUS)
          .description("Pick a date and time")
          .id("schedule"),
      )
      .into()
  });
  let (x, y, _, height) = stage.bounds("schedule");
  let (label, description, icon) = (
    stage.text_quad("Schedule").unwrap(),
    stage.text_quad("Pick a date and time").unwrap(),
    stage.text_quad(&glyph(icons::CALENDAR_PLUS)).unwrap(),
  );
  assert_eq!(stage.text("Schedule").1, 14.0);
  assert_eq!(stage.text("Pick a date and time").1, 12.0);
  assert_eq!(icon.width, 20.0, "a larger icon beside two lines");
  assert_eq!(icon.x - x, 16.0);
  assert_eq!(label.x - (icon.x + icon.width), 12.0);
  assert_eq!(description.x, label.x, "the description starts where the label does");
  assert_eq!(description.y - (label.y + label.height), 2.0);
  assert_eq!(label.y - y, 10.0, "10 above the two lines");
  assert_eq!(height, 10.0 + label.height + 2.0 + description.height + 10.0);
}

#[test]
fn the_arrow_keys_move_the_choice_of_a_segmented_control() {
  let period = Signal::new("week");
  let chosen = period.clone();
  let pressed = Arc::new(Mutex::new(Vec::new()));
  let told = pressed.clone();
  let mut stage = Stage::new(move |_| {
    let after = told.clone();
    Column::new()
      .spacing(8.0)
      .child(SegmentedControl::new(
        chosen.clone(),
        [("day", "Day"), ("week", "Week"), ("month", "Month")],
      ))
      .child(Button::new("After").on_click(move |_| after.lock().unwrap().push("after")))
      .into()
  });
  // Tab comes in at the chosen segment; the arrows move the focus and the choice with it.
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(period.get(), "month");
  stage.key("ArrowRight");
  assert_eq!(period.get(), "day", "past the last is the first");
  stage.key("End");
  assert_eq!(period.get(), "month");
  stage.key("ArrowLeft");
  assert_eq!(period.get(), "week");
  // The control is one stop: the next Tab is on what is after it.
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(*pressed.lock().unwrap(), ["after"]);
}

#[test]
fn the_arrow_keys_move_between_the_toggles_of_a_group_and_space_turns_one() {
  let marks = Signal::new(vec!["bold"]);
  let kept = marks.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .child(ToggleGroup::multiple(
        kept.clone(),
        [("bold", "Bold"), ("italic", "Italic"), ("underline", "Underline")],
      ))
      .into()
  });
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(marks.get(), ["bold"], "moving the focus turns nothing");
  stage.key(" ");
  assert_eq!(marks.get(), ["bold", "italic"]);
  stage.key("ArrowLeft");
  stage.key(" ");
  assert_eq!(marks.get(), ["italic"]);
}
