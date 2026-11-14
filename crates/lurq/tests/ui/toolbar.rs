//! A toolbar: its bar, its controls and the lines between its groups against
//! the board, the keys of one stop of Tab and where the focus comes back to,
//! the tooltip of an icon alone, the controls that do not fit going into the
//! menu at its end, and a theme's rules and renderer.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row, Text},
  layout::{Alignment, layout_kind::ShrinkLimit},
  node::{Element, color::Color},
  ui::{
    Button, Toggle, Toolbar, ToolbarCommand, ToolbarGroup, ToolbarItem, ToolbarPart, ToolbarRenderer, ToolbarVariant,
    ToolbarView, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

/// A button of the bar, its icon alone, which counts its presses in `pressed`.
fn tool(label: &str, icon: &str, pressed: &Arc<AtomicUsize>) -> Button {
  let pressed = pressed.clone();
  Button::new(label)
    .icon(icon)
    .icon_only(true)
    .id(label.to_lowercase())
    .on_click(move |_| {
      pressed.fetch_add(1, Ordering::SeqCst);
    })
}

/// The board's toolbar: Bold (on), Italic and Underline, a line, List, Link and Image.
fn formatting(pressed: &Arc<AtomicUsize>) -> Toolbar {
  Toolbar::new()
    .label("Formatting")
    .id("bar")
    .child(
      ToolbarItem::from(
        Toggle::new("Bold")
          .icon(icons::BOLD)
          .icon_only(true)
          .on(true)
          .id("bold"),
      )
      .shortcut("Ctrl+B"),
    )
    .child(Toggle::new("Italic").icon(icons::ITALIC).icon_only(true).id("italic"))
    .child(
      Toggle::new("Underline")
        .icon(icons::UNDERLINE)
        .icon_only(true)
        .id("underline"),
    )
    .child(Toolbar::separator())
    .child(tool("List", icons::LIST, pressed))
    .child(tool("Link", icons::LINK, pressed))
    .child(tool("Image", icons::IMAGE, pressed))
}

/// `toolbar` in a column `width` wide, in a window of 400 by 300.
fn stage(width: f32, toolbar: impl Fn() -> Toolbar + Send + Sync + 'static) -> Stage {
  Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(Column::new().width(width).child(toolbar().mount(ctx)))
      .into()
  })
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn shift_tab(stage: &mut Stage) {
  stage
    .tree
    .key_down("Tab".to_owned(), "Tab".to_owned(), true, false, false);
  stage
    .tree
    .key_up("Tab".to_owned(), "Tab".to_owned(), true, false, false);
  stage.pass();
}

#[test]
fn the_bar_is_40_high_with_ghost_buttons_of_32_two_apart_and_a_line_between_groups() {
  let pressed = Arc::new(AtomicUsize::new(0));
  let held = pressed.clone();
  let mut stage = stage(360.0, move || formatting(&held));
  let (x, y, _, height) = stage.bounds("bar");
  assert_eq!(height, 40.0, "32 and a padding of 4");
  let (bold_x, bold_y, bold_width, bold_height) = stage.bounds("bold");
  assert_eq!((bold_width, bold_height), (32.0, 32.0));
  assert!(close(bold_x - x, 4.0) && close(bold_y - y, 4.0), "padded 4");
  let (italic_x, ..) = stage.bounds("italic");
  assert!(close(italic_x - (bold_x + 32.0), 2.0), "2 apart");
  // A line of 1 with 4 on either side stands between Underline and List: 2 + 9 + 2.
  let (underline_x, ..) = stage.bounds("underline");
  let (list_x, ..) = stage.bounds("list");
  assert!(close(list_x - (underline_x + 32.0), 13.0), "a group apart");
  let line = stage
    .quads()
    .into_iter()
    .find(|quad| quad.width == 1.0 && quad.height == 20.0 && quad.x > underline_x + 32.0 && quad.x < list_x)
    .expect("the line between the groups");
  assert!(close(line.x, underline_x + 32.0 + 2.0 + 4.0));

  // On a surface: the background's fill, an edge of 1 in the border's color.
  assert_eq!(stage.fill_color("bar"), Some(light(PaletteColor::Background)));
  let edge = stage
    .border("bar")
    .and_then(|quad| quad.border)
    .and_then(|border| border.top);
  assert_eq!(
    edge.map(|side| (side.width, side.color)),
    Some((1.0, light(PaletteColor::Border)))
  );
  // The toggle that is on has the fill of what is picked out; a button at rest has none.
  assert_eq!(stage.fill_color("bold"), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.fill_color("list"), None);
}

#[test]
fn a_floating_bar_has_the_popovers_fill_and_a_plain_one_none() {
  let pressed = Arc::new(AtomicUsize::new(0));
  let held = pressed.clone();
  let mut floating = stage(360.0, move || formatting(&held).variant(ToolbarVariant::Floating));
  assert_eq!(floating.fill_color("bar"), Some(light(PaletteColor::Popover)));
  let held = pressed.clone();
  let mut plain = stage(360.0, move || formatting(&held).variant(ToolbarVariant::Plain));
  assert_eq!(plain.fill_color("bar"), None);
  assert!(plain.border("bar").is_none());
}

#[test]
fn a_press_does_what_the_control_does_and_a_disabled_one_takes_nothing() {
  let pressed = Arc::new(AtomicUsize::new(0));
  let turned = Arc::new(Mutex::new(Vec::new()));
  let (held, told) = (pressed.clone(), turned.clone());
  let mut stage = stage(360.0, move || {
    let told = told.clone();
    Toolbar::new()
      .id("bar")
      .child(
        Toggle::new("Italic")
          .icon(icons::ITALIC)
          .icon_only(true)
          .id("italic")
          .on_change(move |on| told.lock().unwrap().push(on)),
      )
      .child(tool("Link", icons::LINK, &held))
      .child(tool("Image", icons::IMAGE, &held).disabled(true))
  });
  stage.click("italic");
  assert_eq!(*turned.lock().unwrap(), [true]);
  stage.click("link");
  assert_eq!(pressed.load(Ordering::SeqCst), 1);
  stage.click("image");
  assert_eq!(pressed.load(Ordering::SeqCst), 1, "disabled");
}

#[test]
fn the_bar_is_one_stop_whose_arrows_move_and_tab_comes_back_where_the_focus_left() {
  let pressed = Arc::new(AtomicUsize::new(0));
  let held = pressed.clone();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(lurq::components::Button::new("Before").id("before").tab_index(0))
      .child(formatting(&held).mount(ctx))
      .child(lurq::components::Button::new("After").id("after").tab_index(0))
      .into()
  });
  stage.key("Tab");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("bold"), "Tab enters at the first");
  assert!(stage.has_ring(light(PaletteColor::Ring)), "with the ring");
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("italic"));
  stage.key("End");
  assert_eq!(focused(&stage).as_deref(), Some("image"));
  stage.key("Home");
  assert_eq!(focused(&stage).as_deref(), Some("bold"));
  stage.key("ArrowRight");
  stage.key("ArrowRight");
  stage.key("ArrowRight");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("list"),
    "across the line between groups"
  );
  stage.key(" ");
  assert_eq!(pressed.load(Ordering::SeqCst), 1, "Space presses what has the focus");

  // Tab leaves the bar at once; Shift+Tab comes back to the control it left.
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("after"));
  shift_tab(&mut stage);
  assert_eq!(focused(&stage).as_deref(), Some("list"), "back where it left");
}

#[test]
fn the_tooltip_of_an_icon_alone_says_its_label_and_its_shortcut() {
  let pressed = Arc::new(AtomicUsize::new(0));
  let held = pressed.clone();
  let mut stage = stage(360.0, move || formatting(&held));
  assert!(!stage.paints("Bold"));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("bold"));
  stage.pass();
  assert!(stage.paints("Bold"), "the tooltip says its label");
  assert!(stage.paints("Ctrl+B"), "and its shortcut");
}

/// Six icon buttons, A to F, of which F says it stays.
fn letters(pressed: &Arc<Mutex<Vec<&'static str>>>) -> Toolbar {
  let mut bar = Toolbar::new().id("bar").overflow(true);
  for (letter, icon) in [
    ("A", icons::BOLD),
    ("B", icons::ITALIC),
    ("C", icons::UNDERLINE),
    ("D", icons::LIST),
    ("E", icons::LINK),
  ] {
    let pressed = pressed.clone();
    bar = bar.child(
      ToolbarItem::from(
        Button::new(letter)
          .icon(icon)
          .icon_only(true)
          .id(letter.to_lowercase())
          .on_click(move |_| pressed.lock().unwrap().push(letter)),
      )
      .shortcut(&format!("Ctrl+{letter}")),
    );
  }
  bar.child(Toggle::new("F").icon(icons::IMAGE).icon_only(true).id("f"))
}

#[test]
fn a_bar_with_room_for_all_has_no_more_button() {
  let pressed = Arc::new(Mutex::new(Vec::new()));
  let held = pressed.clone();
  let mut stage = stage(360.0, move || letters(&held));
  for id in ["a", "b", "c", "d", "e", "f"] {
    assert_eq!(stage.bounds(id).2, 32.0, "{id} fits");
  }
  let (.., more_width, more_height) = stage.bounds("bar-more");
  assert_eq!((more_width, more_height), (0.0, 0.0), "no More button");
  // 8 of padding, six of 32 and five gaps of 2.
  assert_eq!(stage.bounds("bar").2, 210.0);
}

#[test]
fn what_does_not_fit_goes_into_the_menu_at_the_end_last_first() {
  let pressed = Arc::new(Mutex::new(Vec::new()));
  let held = pressed.clone();
  // 200 wide: A to E and F are 210, so F would go; with the More button and its line (2 + 9 + 2 + 32) E goes too.
  let mut stage = stage(200.0, move || letters(&held));
  for id in ["a", "b", "c", "d"] {
    assert_eq!(stage.bounds(id).2, 32.0, "{id} fits");
  }
  for id in ["e", "f"] {
    assert_eq!(stage.bounds(id).2, 0.0, "{id} is in the menu");
  }
  let (more_x, _, more_width, more_height) = stage.bounds("bar-more");
  assert_eq!((more_width, more_height), (32.0, 32.0));
  let (d_x, ..) = stage.bounds("d");
  assert!(close(more_x - (d_x + 32.0), 2.0 + 9.0 + 2.0), "after a line");

  // More opens the menu of what did not fit, in the order of the bar, with the shortcut of each.
  assert!(!stage.paints("E"));
  stage.click("bar-more");
  assert!(stage.paints("E") && stage.paints("F"));
  assert!(stage.paints("Ctrl+E"));
  assert!(!stage.paints("D"), "what fits is not in the menu");
  let (_, e_y, ..) = stage.bounds("e-item");
  let (_, f_y, ..) = stage.bounds("f-item");
  assert!(e_y < f_y, "in the order of the bar");
  // Choosing an item presses its control and closes the menu.
  stage.click("e-item");
  assert_eq!(*pressed.lock().unwrap(), ["E"]);
  assert!(!stage.paints("Ctrl+E"), "closed");
}

#[test]
fn a_control_that_stays_never_goes_into_the_menu() {
  let pressed = Arc::new(Mutex::new(Vec::new()));
  let held = pressed.clone();
  let mut stage = stage(200.0, move || {
    letters(&held)
      .child(ToolbarItem::from(Button::new("Search").icon(icons::SEARCH).icon_only(true).id("search")).stays(true))
  });
  assert_eq!(stage.bounds("search").2, 32.0, "it stays");
  assert_eq!(stage.bounds("d").2, 0.0, "the others go first");
}

#[test]
fn a_toggle_in_the_menu_is_checked_and_choosing_it_turns_it() {
  let turned = Arc::new(Mutex::new(Vec::new()));
  let told = turned.clone();
  let mut stage = stage(120.0, move || {
    let told = told.clone();
    Toolbar::new()
      .id("bar")
      .overflow(true)
      .child(Button::new("Undo").icon(icons::UNDO).icon_only(true).id("undo"))
      .child(Toolbar::separator())
      .child(
        Toggle::new("Bold")
          .icon(icons::BOLD)
          .icon_only(true)
          .on(true)
          .id("bold")
          .on_change(move |on| told.lock().unwrap().push(on)),
      )
      .child(Toggle::new("Italic").icon(icons::ITALIC).icon_only(true).id("italic"))
  });
  assert_eq!(stage.bounds("bold").2, 0.0, "in the menu");
  stage.click("bar-more");
  assert!(stage.paints("Bold") && stage.paints("Italic"));
  stage.click("bold-item");
  assert_eq!(*turned.lock().unwrap(), [false], "turned off");
}

#[test]
fn a_select_or_field_stays_unless_it_has_an_item_to_stand_for_it() {
  let chosen = Arc::new(AtomicUsize::new(0));
  let told = chosen.clone();
  let mut stage = stage(150.0, move || {
    let told = told.clone();
    Toolbar::new()
      .id("bar")
      .overflow(true)
      .child(ToolbarItem::element(Row::new().id("field").size(60.0, 32.0)))
      .child(ToolbarItem::element(Row::new().id("columns").size(60.0, 32.0)).in_menu(
        ToolbarCommand::new("Columns").id("columns").on_select(move || {
          told.fetch_add(1, Ordering::SeqCst);
        }),
      ))
      .child(Button::new("Export").id("export"))
  });
  assert_eq!(stage.bounds("field").2, 60.0, "a field stays");
  assert_eq!(stage.bounds("columns").2, 0.0, "one with an item goes");
  stage.click("bar-more");
  stage.click("columns-item");
  assert_eq!(chosen.load(Ordering::SeqCst), 1);
}

#[test]
fn a_labelled_group_has_its_label_under_it_and_goes_into_the_menu_whole_under_it() {
  let pressed = Arc::new(AtomicUsize::new(0));
  let held = pressed.clone();
  let mut stage = stage(380.0, move || {
    Toolbar::new()
      .id("bar")
      .variant(ToolbarVariant::Plain)
      .overflow(true)
      .child(
        ToolbarGroup::new()
          .label("Clipboard")
          .child(Button::new("Paste").icon(icons::PASTE).id("paste"))
          .child(Button::new("Cut").icon(icons::CUT).id("cut")),
      )
      .child(
        ToolbarGroup::new()
          .label("Format")
          .child(tool("Strike", icons::STRIKETHROUGH, &held)),
      )
  });
  let label = stage.text_quad("Clipboard").expect("the label");
  let (_, paste_y, _, paste_height) = stage.bounds("paste");
  assert!(label.y > paste_y + paste_height, "under the controls");
  assert_eq!(stage.text("Clipboard").1, 10.0);
  assert_eq!(stage.text("Clipboard").0, light(PaletteColor::MutedForeground));
  drop(stage);

  let held = pressed.clone();
  let mut narrow = self::stage(150.0, move || {
    Toolbar::new()
      .id("bar")
      .overflow(true)
      .child(
        ToolbarGroup::new()
          .label("Clipboard")
          .child(Button::new("Paste").icon(icons::PASTE).id("paste")),
      )
      .child(
        ToolbarGroup::new()
          .label("Format")
          .child(tool("Strike", icons::STRIKETHROUGH, &held)),
      )
  });
  assert_eq!(narrow.bounds("strike").2, 0.0);
  narrow.click("bar-more");
  assert!(narrow.paints("Format"), "under its label in the menu");
  narrow.click("strike-item");
  assert_eq!(pressed.load(Ordering::SeqCst), 1);
}

#[test]
fn the_spacer_pushes_what_follows_to_the_end() {
  let mut stage = stage(360.0, || {
    Toolbar::new()
      .id("bar")
      .child(Button::new("Undo").icon(icons::UNDO).icon_only(true).id("undo"))
      .child(Toolbar::spacer())
      .child(Button::new("Share").id("share"))
  });
  let (x, _, width, _) = stage.bounds("bar");
  let (share_x, _, share_width, _) = stage.bounds("share");
  assert_eq!(width, 360.0);
  assert!(
    close(share_x + share_width, x + width - 4.0),
    "at the end, inside the padding"
  );
}

#[test]
fn a_line_drops_its_marker_while_it_drops_nothing() {
  // The engine's part of the overflow, on its own: a row of 100 with a marker of 20 at its end.
  let row = |width: f32| -> Element {
    let mut row = Row::new().id("row").width(width).align_items(Alignment::Center);
    for index in 0..4 {
      row = row.child(
        Row::new()
          .id(format!("item-{index}"))
          .size(20.0, 10.0)
          .flex_shrink(1.0)
          .shrink_limit(ShrinkLimit::Drop),
      );
    }
    row
      .child(Row::new().id("marker").size(20.0, 10.0).drop_marker(true))
      .into()
  };
  let mut wide = Stage::new(move |_| Column::new().size(400.0, 300.0).child(row(100.0)).into());
  assert_eq!(wide.bounds("item-3").2, 20.0);
  assert_eq!(wide.bounds("marker").2, 0.0, "nothing dropped: no marker");

  let mut narrow = Stage::new(move |_| Column::new().size(400.0, 300.0).child(row(70.0)).into());
  // Without the marker 60 fits three; with it in, two.
  assert_eq!(narrow.bounds("item-1").2, 20.0);
  assert_eq!(narrow.bounds("item-2").2, 0.0);
  assert_eq!(narrow.bounds("item-3").2, 0.0);
  assert_eq!(narrow.bounds("marker").2, 20.0);
}

#[test]
fn a_rule_of_the_theme_changes_the_bar() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Toolbar>(|recipe| recipe.base(ToolbarPart::Root, |s| s.background(PaletteColor::Destructive)));
  let pressed = Arc::new(AtomicUsize::new(0));
  let mut stage = Stage::in_app(app, move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .child(formatting(&pressed).mount(ctx))
      .into()
  });
  assert_eq!(stage.fill_color("bar"), Some(light(PaletteColor::Destructive)));
}

#[test]
fn a_theme_gives_the_bar_another_anatomy() {
  let app = App::new();
  let renderer: ToolbarRenderer = Arc::new(|mut view: ToolbarView| {
    let mut column = Column::new().id("column");
    for index in 0..view.slots.len() {
      column = column.child(view.slot(index));
    }
    column.child(Text::new("Tools")).into()
  });
  app.theme().set_renderer::<Toolbar>(renderer);
  let pressed = Arc::new(AtomicUsize::new(0));
  let mut stage = Stage::in_app(app, move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .child(formatting(&pressed).mount(ctx))
      .into()
  });
  assert!(stage.paints("Tools"));
  let (_, bold_y, ..) = stage.bounds("bold");
  let (_, italic_y, ..) = stage.bounds("italic");
  assert!(italic_y > bold_y, "down a column");
}
