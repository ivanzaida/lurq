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
        ButtonGroup::new([])
          .id("period")
          .button(Button::new("Years").id("years"))
          .button(Button::new("Months").id("months").on_click(move |_| {
            counted.fetch_add(1, Ordering::SeqCst);
          }))
          .button(Button::new("Days").id("days")),
      )
      .child(
        ButtonGroup::new([])
          .variant(ButtonGroupVariant::Primary)
          .id("merge")
          .button(Button::new("Merge").id("action"))
          .button(Button::new("How").icon(icons::CHEVRON_DOWN).icon_only(true).id("how")),
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
      .child(Link::new("Read the docs").external(true).id("docs").on_click(move |_| {
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
  assert!(
    stage.border("docs").is_none(),
    "the link's box, rounded for its ring, has no line"
  );
  // A straight line under the text alone: the label's own bottom edge, with no radius, not under the icon.
  let edge = stage
    .quads()
    .into_iter()
    .find(|quad| quad.border.is_some_and(|border| border.bottom.is_some()))
    .expect("a line under the link the pointer is on");
  let line = edge.border.unwrap();
  assert!(
    line.top.is_none() && line.left.is_none() && line.right.is_none(),
    "under it only"
  );
  let under = line.bottom.expect("the line");
  assert_eq!((under.width, under.color), (1.0, light(PaletteColor::Foreground)));
  assert!(
    edge
      .border_radius
      .is_none_or(|radius| radius.bottom_left == 0.0 && radius.bottom_right == 0.0),
    "straight to its ends"
  );
  assert!(
    (edge.x - label.x).abs() < 0.5 && (edge.width - label.width).abs() < 0.5,
    "under the text"
  );
  assert!(edge.x + edge.width <= icon.x, "not under the icon");
  // The pointer on the icon draws the line under the text too.
  stage
    .tree
    .mouse_move(icon.x + icon.width / 2.0, icon.y + icon.height / 2.0);
  stage.pass();
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| quad.border.is_some_and(|border| border.bottom.is_some()))
  );

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

#[test]
fn a_toggle_with_an_empty_label_and_an_icon_is_a_square_as_an_icon_alone_is() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .spacing(8.0)
      .child(Toggle::new("").icon(icons::BOLD).id("empty"))
      .child(Toggle::new("Bold").icon(icons::BOLD).icon_only(true).id("alone"))
      .into()
  });
  let alone = stage.bounds("alone").2;
  assert_eq!(stage.bounds("empty").2, alone, "no gap and no padding of a label");
}

#[test]
fn a_toggle_narrower_than_its_label_cuts_it() {
  const LONG: &str = "Show the hidden files of every folder";
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(20.0)
      .child(
        Column::new()
          .id("room")
          .width(140.0)
          .child(Toggle::new(LONG).icon(icons::EYE).id("long")),
      )
      .into()
  });
  let (room_x, _, room_width, _) = stage.bounds("room");
  let (x, _, width, _) = stage.bounds("long");
  assert!(x + width <= room_x + room_width + 0.5, "in its room");
  let (text, label_x, label_width) = stage
    .quads()
    .into_iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { text, .. } if text.starts_with("Show") => Some((text.clone(), quad.x, quad.width)),
      _ => None,
    })
    .expect("the label");
  assert!(text.ends_with('…'), "cut with an ellipsis: {text}");
  assert!(label_x + label_width <= x + width + 0.5, "inside the toggle");
}

#[test]
fn a_links_trailing_icon_stands_after_its_text_and_takes_the_place_of_the_external_arrow() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(20.0)
      .spacing(8.0)
      .child(Link::new("Next page").trailing_icon(icons::ARROW_RIGHT))
      .child(Link::new("Out").trailing_icon(icons::LINK).external(true))
      .child(Link::new("Stays").external(false))
      .into()
  });
  let (label, arrow) = (
    stage.text_quad("Next page").unwrap(),
    stage.text_quad(&glyph(icons::ARROW_RIGHT)).unwrap(),
  );
  assert!(arrow.x > label.x + label.width, "after the text");
  assert!(stage.paints(&glyph(icons::LINK)), "its own icon");
  assert!(
    !stage.paints(&glyph(icons::ARROW_UP_RIGHT)),
    "no external arrow beside its own icon, and none for external(false)"
  );
}

#[test]
fn a_fab_takes_a_state_of_the_themes_recipe_and_the_rules_of_its_place() {
  use lurq::{
    app::App,
    ui::{Fab, FabPart, Recipe},
  };
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Fab>(|recipe| recipe.state("urgent", FabPart::Root, |s| s.background(PaletteColor::Destructive)));
  let mut stage = Stage::in_app(app, |_| {
    Row::new()
      .padding(20.0)
      .spacing(8.0)
      .child(Fab::new("Compose", icons::COMPOSE).id("plain"))
      .child(Fab::new("Compose", icons::COMPOSE).state("urgent", true).id("urgent"))
      .child(
        Fab::new("Compose", icons::COMPOSE)
          .rules(&Recipe::new().base(FabPart::Root, |s| s.background(PaletteColor::Success)))
          .id("placed"),
      )
      .into()
  });
  assert_eq!(stage.fill_color("plain"), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color("urgent"), Some(light(PaletteColor::Destructive)));
  assert_eq!(stage.fill_color("placed"), Some(light(PaletteColor::Success)));
}

/// Presses the element whose text is `label`, from the pointer.
fn press_text(stage: &mut Stage, label: &str) {
  let quad = stage.text_quad(label).unwrap_or_else(|| panic!("{label} is painted"));
  let (x, y) = (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  stage.pass();
}

/// Whether the text `label` stands on a fill of `color`: the toggle or segment of it is on.
fn on_fill(stage: &Stage, label: &str, color: Color) -> bool {
  let text = stage.text_quad(label).unwrap();
  stage.quads().into_iter().any(|quad| {
    matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
      && quad.x <= text.x
      && quad.x + quad.width >= text.x + text.width
      && quad.y <= text.y
      && quad.y + quad.height >= text.y + text.height
  })
}

#[test]
fn a_bound_toggle_group_tells_the_new_value_and_not_what_the_app_sets() {
  let (align, marks) = (Signal::new(1u8), Signal::new(vec!["bold"]));
  let (ones, sets) = (Arc::new(Mutex::new(Vec::new())), Arc::new(Mutex::new(Vec::new())));
  let (a, m, o, s) = (align.clone(), marks.clone(), ones.clone(), sets.clone());
  let mut stage = Stage::new(move |_| {
    let (o, s) = (o.clone(), s.clone());
    Column::new()
      .spacing(8.0)
      .child(
        ToggleGroup::new(a.clone(), [(0u8, "Left"), (1, "Center"), (2, "Right")])
          .on_change(move |value: &u8| o.lock().unwrap().push(*value)),
      )
      .child(
        ToggleGroup::multiple(m.clone(), [("bold", "Bold"), ("italic", "Italic")])
          .on_change(move |on: &[&str]| s.lock().unwrap().push(on.to_vec())),
      )
      .into()
  });
  press_text(&mut stage, "Right");
  assert_eq!((align.get(), ones.lock().unwrap().clone()), (2, vec![2]));
  press_text(&mut stage, "Right");
  assert_eq!(*ones.lock().unwrap(), [2], "what is already on is told nothing");
  align.set(0);
  stage.pass();
  assert_eq!(*ones.lock().unwrap(), [2], "what the app sets is told nothing");

  press_text(&mut stage, "Italic");
  press_text(&mut stage, "Bold");
  assert_eq!(
    *sets.lock().unwrap(),
    [vec!["bold", "italic"], vec!["italic"]],
    "the whole new set"
  );
  assert_eq!(marks.get(), ["italic"]);
}

#[test]
fn a_controlled_toggle_group_shows_what_the_app_holds_and_is_told_what_it_is_to_be() {
  let (ones, sets) = (Arc::new(Mutex::new(Vec::new())), Arc::new(Mutex::new(Vec::new())));
  let (o, s) = (ones.clone(), sets.clone());
  let mut stage = Stage::new(move |_| {
    let (o, s) = (o.clone(), s.clone());
    Column::new()
      .spacing(8.0)
      // The app answers nothing: what it holds stays on.
      .child(ToggleGroup::controlled(1u8, [(0u8, "Left"), (1, "Center"), (2, "Right")], move |value| {
        o.lock().unwrap().push(*value)
      }))
      .child(ToggleGroup::controlled_multiple([0u8], [(0u8, "Bold"), (1, "Italic")], move |on| {
        s.lock().unwrap().push(on.to_vec())
      }))
      .into()
  });
  press_text(&mut stage, "Right");
  press_text(&mut stage, "Center");
  assert_eq!(*ones.lock().unwrap(), [2], "the one that is on tells nothing");
  // The pointer away, so that no toggle is under it.
  stage.tree.mouse_move(390.0, 590.0);
  stage.pass();
  assert!(
    on_fill(&stage, "Center", light(PaletteColor::Highlight)),
    "what the app holds"
  );
  assert!(
    !on_fill(&stage, "Right", light(PaletteColor::Highlight)),
    "not what was pressed"
  );
  press_text(&mut stage, "Italic");
  press_text(&mut stage, "Bold");
  assert_eq!(
    *sets.lock().unwrap(),
    [vec![0, 1], vec![]],
    "the whole set it is to be, from what the app holds"
  );
}

#[test]
fn a_press_of_the_chosen_segment_tells_nothing() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let (t, bound) = (told.clone(), Signal::new(0u8));
  let (b, heard) = (bound.clone(), Arc::new(Mutex::new(Vec::new())));
  let h = heard.clone();
  let mut stage = Stage::new(move |_| {
    let (t, h) = (t.clone(), h.clone());
    Column::new()
      .spacing(8.0)
      .child(SegmentedControl::controlled(
        1u8,
        [(0u8, "Day"), (1, "Week")],
        move |value| t.lock().unwrap().push(*value),
      ))
      .child(
        SegmentedControl::new(b.clone(), [(0u8, "Light"), (1, "Dark")])
          .on_change(move |value: &u8| h.lock().unwrap().push(*value)),
      )
      .into()
  });
  press_text(&mut stage, "Week");
  assert!(told.lock().unwrap().is_empty(), "the chosen one");
  press_text(&mut stage, "Day");
  assert_eq!(*told.lock().unwrap(), [0]);
  press_text(&mut stage, "Dark");
  press_text(&mut stage, "Dark");
  assert_eq!((bound.get(), heard.lock().unwrap().clone()), (1, vec![1]));
}

#[test]
fn a_bound_toggle_turns_its_signal_and_then_tells_its_handlers() {
  let (on, told) = (Signal::new(false), Arc::new(Mutex::new(Vec::new())));
  let (bound, t) = (on.clone(), told.clone());
  let mut stage = Stage::new(move |_| {
    let (seen, t) = (bound.clone(), t.clone());
    Row::new()
      .padding(20.0)
      .child(
        Toggle::new("Bold")
          .id("bold")
          .bind(bound.clone())
          .on_change(move |next| t.lock().unwrap().push((next, seen.get_untracked()))),
      )
      .into()
  });
  stage.click("bold");
  assert!(on.get());
  assert_eq!(*told.lock().unwrap(), [(true, true)], "after the signal is set");
  stage.click("bold");
  assert!(!on.get());
}

#[test]
fn a_button_group_is_its_buttons_and_a_disabled_one_takes_nothing() {
  let pressed = Arc::new(AtomicUsize::new(0));
  let counted = pressed.clone();
  let mut stage = Stage::new(move |_| {
    let counted = counted.clone();
    Column::new()
      .padding(20.0)
      .spacing(8.0)
      .child(
        ButtonGroup::new([Button::new("Years").id("years"), Button::new("Months").id("months")])
          .button(Button::new("Days").id("days"))
          .label("Period"),
      )
      .child(
        ButtonGroup::new([Button::new("Off").id("off").on_click(move |_| {
          counted.fetch_add(1, Ordering::SeqCst);
        })])
        .disabled(true),
      )
      .into()
  });
  let (years, days) = (stage.bounds("years"), stage.bounds("days"));
  assert!(days.0 > years.0, "the one added after the list");
  stage.click("off");
  assert_eq!(
    pressed.load(Ordering::SeqCst),
    0,
    "a disabled group's button takes nothing"
  );
}

#[test]
fn a_toggle_group_down_a_column_moves_with_up_and_down() {
  let align = Signal::new(0u8);
  let chosen = align.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(20.0)
      .child(
        ToggleGroup::new(chosen.clone(), [(0u8, "Top"), (1, "Middle"), (2, "Bottom")])
          .orientation(lurq::ui::Orientation::Vertical),
      )
      .into()
  });
  let (top, middle) = (stage.text_quad("Top").unwrap(), stage.text_quad("Middle").unwrap());
  assert!(middle.y > top.y + top.height, "one under the other");
  stage.key("Tab");
  stage.key("ArrowDown");
  stage.key(" ");
  assert_eq!(align.get(), 1);
}

#[test]
fn an_invalid_group_has_a_line_in_the_destructive_color() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(20.0)
      .spacing(8.0)
      .child(ToggleGroup::controlled(0u8, [(0u8, "A"), (1, "B")], |_| {}).invalid(true))
      .into()
  });
  let destructive = Some(light(PaletteColor::Destructive));
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| quad.border.and_then(|border| border.top).map(|side| side.color) == destructive),
    "a line in the destructive color"
  );
}
