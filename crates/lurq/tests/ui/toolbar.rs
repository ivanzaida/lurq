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
    .item(
      ToolbarItem::from(
        Toggle::new("Bold")
          .icon(icons::BOLD)
          .icon_only(true)
          .on(true)
          .id("bold"),
      )
      .shortcut("Ctrl+B"),
    )
    .item(Toggle::new("Italic").icon(icons::ITALIC).icon_only(true).id("italic"))
    .item(
      Toggle::new("Underline")
        .icon(icons::UNDERLINE)
        .icon_only(true)
        .id("underline"),
    )
    .item(Toolbar::separator())
    .item(tool("List", icons::LIST, pressed))
    .item(tool("Link", icons::LINK, pressed))
    .item(tool("Image", icons::IMAGE, pressed))
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
      .item(
        Toggle::new("Italic")
          .icon(icons::ITALIC)
          .icon_only(true)
          .id("italic")
          .on_change(move |on| told.lock().unwrap().push(on)),
      )
      .item(tool("Link", icons::LINK, &held))
      .item(tool("Image", icons::IMAGE, &held).disabled(true))
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
    bar = bar.item(
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
  bar.item(Toggle::new("F").icon(icons::IMAGE).icon_only(true).id("f"))
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
fn more_opened_by_enter_gives_the_focus_to_its_first_item() {
  let pressed = Arc::new(Mutex::new(Vec::new()));
  let held = pressed.clone();
  let mut stage = stage(200.0, move || letters(&held));
  stage.key("Tab");
  stage.key("End");
  assert_eq!(focused(&stage).as_deref(), Some("bar-more"));
  // Enter opens the menu at its first item, and the arrows move in it.
  stage.key("Enter");
  assert!(stage.paints("Ctrl+E"), "open");
  assert_eq!(focused(&stage).as_deref(), Some("e-item"));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("f-item"));
  // Escape closes it and gives the focus back to More.
  stage.key("Escape");
  assert!(!stage.paints("Ctrl+E"), "closed");
  assert_eq!(focused(&stage).as_deref(), Some("bar-more"));
  // Enter on an item presses its control.
  stage.key("Enter");
  stage.key("Enter");
  assert_eq!(*pressed.lock().unwrap(), ["E"]);
}

#[test]
fn a_control_that_stays_never_goes_into_the_menu() {
  let pressed = Arc::new(Mutex::new(Vec::new()));
  let held = pressed.clone();
  let mut stage = stage(200.0, move || {
    letters(&held)
      .item(ToolbarItem::from(Button::new("Search").icon(icons::SEARCH).icon_only(true).id("search")).stays(true))
  });
  assert_eq!(stage.bounds("search").2, 32.0, "it stays");
  assert_eq!(stage.bounds("d").2, 0.0, "the others go first");
}

#[test]
fn a_toggle_in_the_menu_is_checked_and_choosing_it_turns_it() {
  let turned = Arc::new(Mutex::new(Vec::new()));
  let told = turned.clone();
  // 100 wide: Undo, the line and Bold, and Italic are 111 with their padding and gaps; with the More button and
  // its line, Bold goes after Italic.
  let mut stage = stage(100.0, move || {
    let told = told.clone();
    Toolbar::new()
      .id("bar")
      .overflow(true)
      .item(Button::new("Undo").icon(icons::UNDO).icon_only(true).id("undo"))
      .item(Toolbar::separator())
      .item(
        Toggle::new("Bold")
          .icon(icons::BOLD)
          .icon_only(true)
          .on(true)
          .id("bold")
          .on_change(move |on| told.lock().unwrap().push(on)),
      )
      .item(Toggle::new("Italic").icon(icons::ITALIC).icon_only(true).id("italic"))
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
      .item(ToolbarItem::element(Row::new().id("field").size(60.0, 32.0)))
      .item(ToolbarItem::element(Row::new().id("columns").size(60.0, 32.0)).in_menu(
        ToolbarCommand::new("Columns").id("columns").on_select(move || {
          told.fetch_add(1, Ordering::SeqCst);
        }),
      ))
      .item(Button::new("Export").id("export"))
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
      .item(
        ToolbarGroup::new()
          .label("Clipboard")
          .item(Button::new("Paste").icon(icons::PASTE).id("paste"))
          .item(Button::new("Cut").icon(icons::CUT).id("cut")),
      )
      .item(
        ToolbarGroup::new()
          .label("Format")
          .item(tool("Strike", icons::STRIKETHROUGH, &held)),
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
      .item(
        ToolbarGroup::new()
          .label("Clipboard")
          .item(Button::new("Paste").icon(icons::PASTE).id("paste")),
      )
      .item(
        ToolbarGroup::new()
          .label("Format")
          .item(tool("Strike", icons::STRIKETHROUGH, &held)),
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
      .item(Button::new("Undo").icon(icons::UNDO).icon_only(true).id("undo"))
      .item(Toolbar::spacer())
      .item(Button::new("Share").id("share"))
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

#[test]
fn a_disabled_bar_takes_nothing_and_its_menu_and_more_neither() {
  let pressed = Arc::new(Mutex::new(Vec::new()));
  let held = pressed.clone();
  let mut stage = stage(200.0, move || letters(&held).disabled(true).more_label("Other tools"));
  stage.click("a");
  assert!(pressed.lock().unwrap().is_empty(), "a button of a disabled bar");
  stage.click("bar-more");
  assert!(!stage.paints("Ctrl+E"), "More of a disabled bar opens nothing");
}

#[test]
fn more_is_called_by_its_label_in_its_tooltip() {
  let pressed = Arc::new(Mutex::new(Vec::new()));
  let held = pressed.clone();
  let mut stage = stage(200.0, move || letters(&held).more_label("Other tools"));
  stage.key("Tab");
  stage.key("End");
  assert_eq!(focused(&stage).as_deref(), Some("bar-more"));
  assert!(stage.paints("Other tools"), "the tooltip says the label");
  assert!(!stage.paints("More"));
}

#[test]
fn a_command_runs_every_handler_it_is_given_in_order() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let held = told.clone();
  let mut stage = stage(120.0, move || {
    let (first, second) = (held.clone(), held.clone());
    Toolbar::new()
      .id("bar")
      .overflow(true)
      .item(Button::new("Undo").icon(icons::UNDO).icon_only(true).id("undo"))
      .item(Button::new("Redo").icon(icons::REDO).icon_only(true).id("redo"))
      .item(
        ToolbarItem::element(Text::new("Columns of the table")).in_menu(
          ToolbarCommand::new("Columns")
            .id("columns")
            .on_select(move || first.lock().unwrap().push("first"))
            .on_select(move || second.lock().unwrap().push("second")),
        ),
      )
  });
  stage.click("bar-more");
  stage.click("columns-item");
  assert_eq!(*told.lock().unwrap(), ["first", "second"]);
}

#[test]
fn a_state_of_the_themes_recipe_and_a_forced_more_button() {
  let app = App::new();
  app.theme().patch_recipe::<Toolbar>(|recipe| {
    recipe.state("loud", ToolbarPart::Root, |s| s.background(PaletteColor::Destructive))
  });
  let pressed = Arc::new(Mutex::new(Vec::new()));
  let held = pressed.clone();
  let mut stage = Stage::in_app(app, move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(
        Column::new().width(200.0).child(
          letters(&held)
            .state("loud", true)
            .force(Some(lurq::ui::Interaction::Hovered))
            .mount(ctx),
        ),
      )
      .into()
  });
  assert_eq!(stage.fill_color("bar"), Some(light(PaletteColor::Destructive)));
  assert_eq!(
    stage.fill_color("bar-more"),
    Some(light(PaletteColor::Highlight)),
    "More as under the pointer"
  );
}

#[test]
fn a_bar_that_overflows_with_room_for_all_ends_its_last_control_at_its_padding() {
  // With overflow, a bar wide enough for everything shows no More button, and keeps no room for it: after a spacer
  // its last control ends at the bar's end less its padding, and without one the bar is as wide as its controls.
  let mut stage = stage(360.0, || {
    Toolbar::new()
      .id("bar")
      .overflow(true)
      .item(Button::new("Undo").icon(icons::UNDO).icon_only(true).id("undo"))
      .item(Toolbar::spacer())
      .item(Button::new("Share").id("share"))
  });
  let (x, _, width, _) = stage.bounds("bar");
  let (share_x, _, share_width, _) = stage.bounds("share");
  assert_eq!(width, 360.0);
  assert!(
    close(share_x + share_width, x + width - 4.0),
    "at the end, inside the padding: ends at {} in a bar ending at {}",
    share_x + share_width,
    x + width
  );
  assert_eq!(stage.bounds("bar-more").2, 0.0, "no More button");
}

#[test]
fn a_dropped_marker_leaves_its_room_to_what_grows() {
  // A row of 100, 10 apart: an item of 20, a child that grows, and a marker of 20. Nothing is dropped, so the
  // marker is not there, and what grows has its room and the spacing beside it: 100 less the item and one gap.
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new()
          .id("row")
          .width(100.0)
          .spacing(10.0)
          .child(
            Row::new()
              .id("item")
              .size(20.0, 10.0)
              .flex_shrink(1.0)
              .shrink_limit(ShrinkLimit::Drop),
          )
          .child(Row::new().id("grows").height(10.0).flex(1.0))
          .child(Row::new().id("marker").size(20.0, 10.0).drop_marker(true)),
      )
      .into()
  });
  assert_eq!(stage.bounds("marker").2, 0.0, "nothing dropped: no marker");
  assert_eq!(
    stage.bounds("grows").2,
    70.0,
    "the marker's room and its gap are what grows"
  );
}

/// A plain bar of three labelled groups, each of two icons, in a frame whose width is `width`, as a person drags it.
fn grouped(width: lurq::core::Signal<f32>) -> Stage {
  let pressed = Arc::new(AtomicUsize::new(0));
  let mut stage = Stage::new(move |ctx| {
    let group = |label: &str, tools: [(&str, &str); 2]| {
      ToolbarGroup::new()
        .label(label)
        .item(tool(tools[0].0, tools[0].1, &pressed))
        .item(tool(tools[1].0, tools[1].1, &pressed))
    };
    let bar = Toolbar::new()
      .id("bar")
      .variant(ToolbarVariant::Plain)
      .overflow(true)
      .item(group(
        "Text",
        [("Strike", icons::STRIKETHROUGH), ("Underline", icons::UNDERLINE)],
      ))
      .item(group("Lists", [("List", icons::LIST), ("Bold", icons::BOLD)]))
      .item(group("Insert", [("Link", icons::LINK), ("Image", icons::IMAGE)]));
    Column::new()
      .size(700.0, 600.0)
      .padding(20.0)
      .child(Column::new().width(width.get()).child(bar.mount(ctx)))
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(lurq::layout::Constraints::loose(lurq::layout::Size::new(
      700.0, 600.0,
    ))));
  stage.pass();
  stage
}

const GROUPS: [(&str, [&str; 2]); 3] = [
  ("Text", ["strike", "underline"]),
  ("Lists", ["list", "bold"]),
  ("Insert", ["link", "image"]),
];

/// Checks the grouped bar at `width`, its menu opened by a press and closed by Escape: the groups shown are the first
/// ones, apart from each other and from the More button, and the others are in its menu under their labels.
fn check_grouped(stage: &mut Stage, width: f32, how: &str) {
  let mut spans = Vec::new();
  for (label, ids) in GROUPS {
    let rects: Vec<_> = ids.iter().map(|id| stage.bounds(id)).collect();
    let shown = rects.iter().filter(|rect| rect.2 > 0.0).count();
    assert!(
      shown == 0 || shown == 2,
      "{how} at {width}: {label} is in the bar or the menu whole: {rects:?}"
    );
    spans.push((label, (shown > 0).then(|| (rects[0].0, rects[1].0 + rects[1].2))));
  }
  assert!(
    spans
      .iter()
      .skip_while(|(_, span)| span.is_some())
      .all(|(_, span)| span.is_none()),
    "{how} at {width}: the last groups go first: {spans:?}"
  );
  let shown: Vec<_> = spans
    .iter()
    .filter_map(|(label, span)| span.map(|span| (*label, span)))
    .collect();
  for pair in shown.windows(2) {
    assert!(
      pair[0].1.1 <= pair[1].1.0 + 0.5,
      "{how} at {width}: {} runs over {}: {spans:?}",
      pair[0].0,
      pair[1].0
    );
  }
  let (more_x, _, more_width, _) = stage.bounds("bar-more");
  let dropped: Vec<_> = spans
    .iter()
    .filter(|(_, span)| span.is_none())
    .map(|(label, _)| *label)
    .collect();
  if dropped.is_empty() {
    assert_eq!(more_width, 0.0, "{how} at {width}: no More button while all fit");
    return;
  }
  assert!(more_width > 0.0, "{how} at {width}: the More button is there");
  if let Some((label, (_, end))) = shown.last() {
    assert!(
      *end <= more_x + 0.5,
      "{how} at {width}: {label} runs under the More button"
    );
  }
  stage.click("bar-more");
  for label in &dropped {
    assert!(stage.paints(label), "{how} at {width}: {label} is in the menu");
  }
  stage.key("Escape");
  stage.pass();
}

#[test]
fn at_every_width_a_grouped_bars_groups_stand_apart_and_the_rest_are_in_the_menu() {
  let widths: Vec<f32> = (0..=28).map(|step| 600.0 - 20.0 * step as f32).collect();
  for &width in &widths {
    let mut stage = grouped(lurq::core::Signal::new(width));
    for _ in 0..3 {
      stage.pass();
    }
    check_grouped(&mut stage, width, "built");
  }
  let width = lurq::core::Signal::new(600.0f32);
  let mut stage = grouped(width.clone());
  for &at in widths.iter().chain(widths.iter().rev()) {
    width.set(at);
    for _ in 0..3 {
      stage.pass();
    }
    check_grouped(&mut stage, at, "dragged");
  }
}

#[test]
fn a_group_in_the_menu_stays_there_after_escape_closes_the_menu_a_press_opened() {
  let mut stage = grouped(lurq::core::Signal::new(200.0));
  for _ in 0..3 {
    stage.pass();
  }
  assert_eq!(stage.bounds("link").2, 0.0, "Insert went into the menu");
  stage.click("bar-more");
  stage.key("Escape");
  stage.pass();
  assert_eq!(
    stage.bounds("link").2,
    0.0,
    "and is not drawn in the bar once the menu is closed"
  );
  assert!(!stage.paints("Insert"), "nor its label");
}

/// The grouped bar's scene, `width` wide, in a window of 700 by 600.
fn grouped_scene(width: lurq::core::Signal<f32>) -> impl Fn(&mut lurq::app::ctx::Ctx) -> Element + Send + Sync + 'static {
  let pressed = Arc::new(AtomicUsize::new(0));
  move |ctx| {
    let group = |label: &str, tools: [(&str, &str); 2]| {
      ToolbarGroup::new()
        .label(label)
        .item(tool(tools[0].0, tools[0].1, &pressed))
        .item(tool(tools[1].0, tools[1].1, &pressed))
    };
    let bar = Toolbar::new()
      .id("bar")
      .variant(ToolbarVariant::Plain)
      .overflow(true)
      .item(group("Text", [("Strike", icons::STRIKETHROUGH), ("Underline", icons::UNDERLINE)]))
      .item(group("Lists", [("List", icons::LIST), ("Bold", icons::BOLD)]))
      .item(group("Insert", [("Link", icons::LINK), ("Image", icons::IMAGE)]));
    Column::new()
      .size(700.0, 600.0)
      .padding(20.0)
      .child(Column::new().width(width.get()).child(bar.mount(ctx)))
      .into()
  }
}

#[test]
fn the_first_frame_at_a_width_is_the_settled_one() {
  let widths = [600.0, 300.0, 220.0, 160.0, 100.0, 60.0];
  for width in widths {
    let mut stage = Stage::first_frame(700.0, 600.0, grouped_scene(lurq::core::Signal::new(width)));
    let first = stage.picture();
    stage.frame();
    assert_eq!(first, stage.picture(), "built at {width}: the first frame is the next one");
  }
  let width = lurq::core::Signal::new(600.0f32);
  let mut stage = Stage::first_frame(700.0, 600.0, grouped_scene(width.clone()));
  stage.frame();
  for at in widths.iter().chain(widths.iter().rev()) {
    width.set(*at);
    stage.frame();
    let first = stage.picture();
    stage.frame();
    assert_eq!(first, stage.picture(), "dragged to {at}: the first frame is the next one");
  }
}
