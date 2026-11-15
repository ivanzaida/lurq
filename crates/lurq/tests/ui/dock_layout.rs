//! A dock layout: the board's bars and strip, which bar a group has, a press of a tab, the app's state shown and not
//! told, a drag onto a zone, an edge, nothing and outside the window, Escape, a whole group dragged, the menus, F6
//! between groups, the strip and its panel, maximize, close and its refusal, the empty area of documents, what is
//! kept while a panel moves, what tooling reads, and a theme's rules.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    ctx::Ctx,
    events::{MouseButton, ScrollPhase},
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row, ScrollVertical, Text, TextInput},
  core::Signal,
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{DockLayout, DockLayoutPart, DockNode, DockPanel, DockPlace, DockSide, DockState},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The board's layout: the explorer at the start, two documents, the terminal's group of three along the bottom, and
/// the outline hidden in a strip at the right.
fn board() -> DockState {
  let mut state = DockState::new(
    DockNode::row([
      DockNode::group(["explorer"]),
      DockNode::column([
        DockNode::documents(DockNode::group(["blur.rs", "theme.rs"])),
        DockNode::group(["terminal", "problems", "output"]),
      ])
      .shares([0.7, 0.3]),
    ])
    .shares([0.25, 0.75]),
  );
  state.show("outline", DockPlace::Hidden(DockSide::Right));
  state
}

fn panels(notes: &Signal<String>) -> Vec<DockPanel> {
  let rows = Column::new().with_children((0..40).map(|row| {
    Row::new()
      .height(20.0)
      .id(format!("row-{row}"))
      .child(Text::new(&format!("line {row}")))
  }));
  vec![
    DockPanel::new("explorer", "Explorer")
      .child(TextInput::new(notes.clone()).id("notes"))
      .child(ScrollVertical::new(rows).height(120.0).id("scroll")),
    DockPanel::new("blur.rs", "blur.rs").child(Text::new("fn blur() {}")),
    DockPanel::new("theme.rs", "theme.rs").child(Text::new("fn theme() {}")),
    DockPanel::new("terminal", "Terminal").child(Text::new("$ cargo run -p demo")),
    DockPanel::new("problems", "Problems")
      .count(3)
      .child(Text::new("3 problems")),
    DockPanel::new("output", "Output").child(Text::new("output")),
    DockPanel::new("outline", "Outline").child(Text::new("the outline")),
  ]
}

/// A window of 860 by 440, the board's, with the layout bound to `state`, its id `dock`; `told` hears every change.
fn scene(state: &Signal<DockState>, notes: &Signal<String>, told: &Arc<Mutex<Vec<DockState>>>) -> Stage {
  let (state, notes, told) = (state.clone(), notes.clone(), told.clone());
  let mut stage = Stage::new(move |ctx: &mut Ctx| -> Element {
    let told = told.clone();
    Column::new()
      .size(860.0, 440.0)
      .child(
        DockLayout::new(state.clone(), panels(&notes))
          .id("dock")
          .on_change(move |next| told.lock().unwrap().push(next.clone()))
          .mount(ctx),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(lurq::layout::Constraints::loose(lurq::layout::Size::new(
      860.0, 440.0,
    ))));
  stage.tree.resize(860, 440);
  stage.pass();
  stage.pass();
  stage
}

fn fresh() -> (Signal<DockState>, Signal<String>, Arc<Mutex<Vec<DockState>>>, Stage) {
  let (state, notes, told) = (
    Signal::new(board()),
    Signal::new("abc".to_owned()),
    Arc::new(Mutex::new(Vec::new())),
  );
  let stage = scene(&state, &notes, &told);
  (state, notes, told, stage)
}

fn press_at(stage: &mut Stage, (x, y): (f32, f32)) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

/// Presses at `from` and moves the pointer to `to` in steps, holding it.
fn hold_to(stage: &mut Stage, from: (f32, f32), to: (f32, f32)) {
  stage.tree.mouse_move(from.0, from.1);
  stage.tree.mouse_down(from.0, from.1, MouseButton::Left);
  for step in 1..=6 {
    let t = step as f32 / 6.0;
    stage
      .tree
      .mouse_move(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
    stage.pass();
  }
}

fn release(stage: &mut Stage, (x, y): (f32, f32)) {
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  stage.pass();
}

/// The middle of the body of the group whose first panel is `first`.
fn body_middle(stage: &mut Stage, first: &str, bar: f32) -> (f32, f32) {
  let (x, y, width, height) = stage.bounds(&format!("dock-group-{first}"));
  (x + width / 2.0, y + bar + (height - bar) / 2.0)
}

/// The squares of 32 painted in `color`: the indicator's zones.
fn zones(stage: &Stage, color: Color) -> usize {
  stage
    .quads()
    .iter()
    .filter(|quad| {
      (quad.width - 32.0).abs() < 0.5
        && (quad.height - 32.0).abs() < 0.5
        && matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
    })
    .count()
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn at_rest_it_is_the_boards_layout_with_its_bars_and_its_strip() {
  let (_, _, _, mut stage) = fresh();
  let (_, _, width, height) = stage.bounds("dock");
  assert_eq!((width, height), (860.0, 440.0));
  // The bottom group of three: tabs in capitals on a bar of 30 on the muted fill, the one shown in the foreground.
  let (_, _, _, tab_height) = stage.bounds("dock-tab-terminal");
  assert_eq!(tab_height, 30.0);
  assert_eq!(stage.text("TERMINAL").0, light(PaletteColor::Foreground));
  assert_eq!(stage.text("PROBLEMS").0, light(PaletteColor::MutedForeground));
  assert!(stage.paints("3"), "the count after Problems");
  assert!(stage.bounds("dock-maximize-terminal").2 == 22.0);
  assert!(stage.bounds("dock-close-terminal").2 == 22.0);
  // The documents: the board's compact tabs, 32 high, the one shown on the background.
  assert_eq!(stage.bounds("dock-tab-blur.rs").3, 32.0);
  assert!(stage.paints("blur.rs") && stage.paints("theme.rs"));
  // The explorer, a group of one, has the panel's header: its title in capitals.
  assert!(stage.paints("EXPLORER"));
  // The outline is hidden in a strip of 30 at the right.
  let (sx, _, sw, sh) = stage.bounds("dock-strip-outline");
  assert_eq!((sw, sh), (22.0, 22.0));
  assert!(sx > 860.0 - 30.0 && sx < 860.0 - 4.0);
  assert!(!stage.paints("the outline"), "what a hidden panel holds is not shown");
}

#[test]
fn a_press_of_a_tab_shows_its_panel_and_tells_the_app_and_the_apps_state_tells_nobody() {
  let (state, _, told, mut stage) = fresh();
  assert!(stage.paints("$ cargo run -p demo"));
  stage.click("dock-tab-problems");
  assert!(stage.paints("3 problems"));
  assert!(!stage.paints("$ cargo run -p demo"));
  assert_eq!(
    state
      .get_untracked()
      .group_of("problems")
      .and_then(|group| group.shown.clone())
      .as_deref(),
    Some("problems")
  );
  assert_eq!(told.lock().unwrap().len(), 1);
  // The app sets the state: shown, and nobody told.
  let mut next = state.get_untracked();
  next.select("output");
  state.set(next);
  stage.pass();
  assert!(stage.paints("output"));
  assert_eq!(told.lock().unwrap().len(), 1);
}

#[test]
fn a_state_naming_a_panel_the_app_does_not_give_leaves_it_out_and_an_unnamed_panel_is_not_shown() {
  let state = Signal::new(DockState::new(DockNode::row([
    DockNode::group(["ghost", "terminal"]),
    DockNode::group(["blur.rs"]),
  ])));
  let notes = Signal::new(String::new());
  let told = Arc::new(Mutex::new(Vec::new()));
  let mut stage = scene(&state, &notes, &told);
  assert!(stage.paints("TERMINAL") || stage.paints("Terminal".to_uppercase().as_str()));
  assert!(!stage.paints("GHOST"));
  assert!(
    !stage.paints("EXPLORER"),
    "a panel the state does not name is not shown"
  );
  assert!(stage.tree.get_element_by_id_mut("dock-group-ghost").is_none());
}

#[test]
fn a_tab_dragged_onto_a_zone_shows_the_indicator_and_the_preview_and_lands_beside_the_group() {
  let (state, _, told, mut stage) = fresh();
  let (tx, ty, tw, th) = stage.bounds("dock-tab-terminal");
  let from = (tx + tw / 2.0, ty + th / 2.0);
  let (mx, my) = body_middle(&mut stage, "blur.rs", 32.0);
  let right = (mx + 32.0, my);
  hold_to(&mut stage, from, right);
  // Five zones over the group and four on the layout's edges; the one under the pointer in the info color.
  assert_eq!(zones(&stage, light(PaletteColor::Info)), 1);
  assert_eq!(zones(&stage, light(PaletteColor::Popover)), 8);
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::InfoMuted))),
    "the preview of where it lands"
  );
  assert!(told.lock().unwrap().is_empty(), "nothing moves while it is dragged");
  release(&mut stage, right);
  let now = state.get_untracked();
  assert!(
    now.in_documents("terminal"),
    "beside a document, in the area of documents"
  );
  assert_ne!(
    now.group_of("terminal").map(|group| group.panels.clone()),
    now.group_of("blur.rs").map(|group| group.panels.clone()),
    "a group of its own"
  );
  assert_eq!(zones(&stage, light(PaletteColor::Info)), 0, "the indicator goes");
  assert_eq!(told.lock().unwrap().len(), 1);
}

#[test]
fn a_tab_let_go_on_the_middle_zone_becomes_a_tab_of_that_group() {
  let (state, _, _, mut stage) = fresh();
  let (tx, ty, tw, th) = stage.bounds("dock-tab-output");
  let middle = body_middle(&mut stage, "blur.rs", 32.0);
  hold_to(&mut stage, (tx + tw / 2.0, ty + th / 2.0), middle);
  release(&mut stage, middle);
  let now = state.get_untracked();
  assert_eq!(
    now.group_of("output").map(|group| group.panels.clone()),
    Some(vec!["blur.rs".to_owned(), "theme.rs".to_owned(), "output".to_owned()])
  );
}

#[test]
fn a_tab_let_go_on_an_edge_square_docks_along_the_layouts_edge() {
  let (state, _, _, mut stage) = fresh();
  let (tx, ty, tw, th) = stage.bounds("dock-tab-output");
  // The square of the top edge: 8 in from the edge, 32 square, in the middle of the layout's width.
  let top = (430.0, 8.0 + 16.0);
  hold_to(&mut stage, (tx + tw / 2.0, ty + th / 2.0), top);
  assert_eq!(zones(&stage, light(PaletteColor::Info)), 1);
  release(&mut stage, top);
  let now = state.get_untracked();
  match &now.root {
    Some(DockNode::Split(split)) => {
      assert_eq!(split.orientation, lurq::ui::Orientation::Vertical);
      assert!(matches!(&split.children[0], DockNode::Group(group) if group.panels == ["output"]));
    }
    other => panic!("a column with the output on top: {other:?}"),
  }
}

#[test]
fn let_go_over_nothing_it_floats_where_it_was_let_go_and_escape_gives_a_drag_up() {
  let (state, _, told, mut stage) = fresh();
  let (tx, ty, tw, th) = stage.bounds("dock-tab-output");
  let from = (tx + tw / 2.0, ty + th / 2.0);
  // Escape first: nothing moves.
  hold_to(&mut stage, from, (500.0, 120.0));
  stage.key("Escape");
  assert_eq!(
    zones(&stage, light(PaletteColor::Popover)),
    0,
    "the indicator goes at once"
  );
  release(&mut stage, (500.0, 120.0));
  assert!(told.lock().unwrap().is_empty());
  assert_eq!(state.get_untracked(), board().valid(|_| true));
  // Over the documents' body but on no zone: it floats there.
  let (bx, by) = body_middle(&mut stage, "blur.rs", 32.0);
  let nowhere = (bx - 120.0, by - 60.0);
  hold_to(&mut stage, from, nowhere);
  release(&mut stage, nowhere);
  let now = state.get_untracked();
  assert!(now.is_floating("output"));
  let float = now
    .floating
    .iter()
    .find(|float| float.group.panels == ["output"])
    .unwrap();
  assert!(
    (float.position.0 - (nowhere.0 - 24.0)).abs() < 2.0,
    "{:?}",
    float.position
  );
  assert!(stage.paints("Output") || stage.paints("output"));
}

#[test]
fn let_go_outside_the_window_it_is_told_to_the_app_and_nothing_moves() {
  let state = Signal::new(board());
  let notes = Signal::new(String::new());
  let detached = Arc::new(Mutex::new(Vec::<String>::new()));
  let (bound, heard) = (state.clone(), detached.clone());
  let mut stage = Stage::new(move |ctx: &mut Ctx| -> Element {
    let heard = heard.clone();
    Column::new()
      .size(860.0, 440.0)
      .child(
        DockLayout::new(bound.clone(), panels(&notes))
          .id("dock")
          .on_detach(move |panel| heard.lock().unwrap().push(panel.to_owned()))
          .mount(ctx),
      )
      .into()
  });
  stage.tree.resize(860, 440);
  stage.pass();
  stage.pass();
  let (tx, ty, tw, th) = stage.bounds("dock-tab-output");
  hold_to(&mut stage, (tx + tw / 2.0, ty + th / 2.0), (900.0, 500.0));
  release(&mut stage, (900.0, 500.0));
  assert_eq!(*detached.lock().unwrap(), ["output"]);
  assert_eq!(state.get_untracked(), board());
}

#[test]
fn a_whole_group_is_dragged_by_its_bars_empty_room() {
  let (state, _, _, mut stage) = fresh();
  // The empty room of the bottom group's bar: after its tabs, before its buttons.
  let (mx, my, _, mh) = stage.bounds("dock-maximize-terminal");
  let from = (mx - 60.0, my + mh / 2.0);
  let (bx, by) = body_middle(&mut stage, "explorer", 30.0);
  hold_to(&mut stage, from, (bx, by));
  release(&mut stage, (bx, by));
  let now = state.get_untracked();
  assert_eq!(
    now.group_of("terminal").map(|group| group.panels.clone()),
    Some(vec![
      "explorer".to_owned(),
      "terminal".to_owned(),
      "problems".to_owned(),
      "output".to_owned()
    ]),
    "the three, after the explorer's tab"
  );
}

#[test]
fn the_tabs_menu_moves_floats_hides_maximizes_and_closes_from_the_keyboard() {
  let (state, _, _, mut stage) = fresh();
  stage.click("dock-tab-output");
  // Shift+F10 opens the tab's menu.
  stage
    .tree
    .key_down("F10".to_owned(), "F10".to_owned(), true, false, false);
  stage
    .tree
    .key_up("F10".to_owned(), "F10".to_owned(), true, false, false);
  stage.pass();
  assert!(stage.paints("Move to the left") && stage.paints("Float") && stage.paints("Hide"));
  assert!(stage.paints("Maximize") && stage.paints("Close"));
  let left = stage.text_quad("Move to the left").unwrap();
  press_at(&mut stage, (left.x + 4.0, left.y + left.height / 2.0));
  let now = state.get_untracked();
  match &now.root {
    Some(DockNode::Split(split)) => assert!(
      matches!(&split.children[0], DockNode::Group(group) if group.panels == ["output"]),
      "the output along the left: {split:?}"
    ),
    other => panic!("{other:?}"),
  }
}

#[test]
fn f6_goes_from_group_to_group_and_back() {
  let (_, _, _, mut stage) = fresh();
  stage.click("dock-tab-terminal");
  stage.key("F6");
  let first = focused(&stage);
  stage.key("F6");
  let second = focused(&stage);
  assert_ne!(first, second, "each F6 goes to another group");
  stage
    .tree
    .key_down("F6".to_owned(), "F6".to_owned(), true, false, false);
  stage.tree.key_up("F6".to_owned(), "F6".to_owned(), true, false, false);
  stage.pass();
  assert_eq!(focused(&stage), first, "and Shift+F6 back");
}

#[test]
fn a_press_of_a_strip_button_shows_the_panel_beside_it_and_a_press_elsewhere_escape_or_put_back_hide_it() {
  let (state, _, _, mut stage) = fresh();
  stage.click("dock-strip-outline");
  assert!(stage.paints("the outline"));
  let (fx, _, fw, _) = stage.bounds("dock-flyout-outline");
  assert!((fx + fw - (860.0 - 30.0)).abs() < 1.5, "beside the strip");
  assert_eq!(fw, 280.0);
  // A press elsewhere hides it.
  press_at(&mut stage, (300.0, 200.0));
  assert!(!stage.paints("the outline"));
  // Escape hides it too.
  stage.click("dock-strip-outline");
  assert!(stage.paints("the outline"));
  stage.key("Escape");
  assert!(!stage.paints("the outline"));
  // Put back: in its group again, and the strip goes.
  stage.click("dock-strip-outline");
  let pin = stage.tree.get_element_by_id_mut("dock-flyout-outline").is_some();
  assert!(pin);
  let put_back = stage.text_quad("OUTLINE").map(|quad| (quad.x, quad.y)).unwrap();
  let (fx, fy, fw, _) = stage.bounds("dock-flyout-outline");
  // The header's first button, Put back, before the close at its end.
  press_at(&mut stage, (fx + fw - 4.0 - 22.0 - 11.0, fy + 15.0));
  let _ = put_back;
  assert_eq!(state.get_untracked().is_hidden("outline"), None);
  assert!(stage.paints("the outline"));
}

#[test]
fn maximize_fills_the_layout_with_the_group_and_restore_gives_it_its_place() {
  let (state, _, _, mut stage) = fresh();
  stage.click("dock-maximize-terminal");
  assert_eq!(state.get_untracked().maximized.as_deref(), Some("terminal"));
  assert!(!stage.paints("blur.rs"), "the rest is out of sight");
  let (_, _, width, height) = stage.bounds("dock-group-terminal");
  assert!(width > 850.0 && height > 430.0, "{width} × {height}");
  stage.click("dock-maximize-terminal");
  assert_eq!(state.get_untracked().maximized, None);
  assert!(stage.paints("blur.rs"));
}

#[test]
fn close_takes_the_panel_out_unless_its_handler_refuses() {
  let state = Signal::new(board());
  let notes = Signal::new(String::new());
  let refuse = Arc::new(Mutex::new(true));
  let (bound, refusing) = (state.clone(), refuse.clone());
  let mut stage = Stage::new(move |ctx: &mut Ctx| -> Element {
    let refusing = refusing.clone();
    let panels = panels(&notes).into_iter().map(move |panel| {
      let refusing = refusing.clone();
      panel.on_close(move |event| {
        if *refusing.lock().unwrap() {
          event.prevent_default();
        }
      })
    });
    Column::new()
      .size(860.0, 440.0)
      .child(DockLayout::new(bound.clone(), panels).id("dock").mount(ctx))
      .into()
  });
  stage.tree.resize(860, 440);
  stage.pass();
  stage.pass();
  stage.click("dock-close-terminal");
  assert!(state.get_untracked().contains("terminal"), "refused");
  *refuse.lock().unwrap() = false;
  stage.click("dock-close-terminal");
  assert!(!state.get_untracked().contains("terminal"));
  assert!(stage.paints("PROBLEMS"), "its group shows the next");
}

#[test]
fn an_empty_area_of_documents_shows_what_the_app_gives_for_it() {
  let state = Signal::new(DockState::new(DockNode::row([
    DockNode::group(["explorer"]),
    DockNode::documents(None),
  ])));
  let notes = Signal::new(String::new());
  let bound = state.clone();
  let mut stage = Stage::new(move |ctx: &mut Ctx| -> Element {
    Column::new()
      .size(860.0, 440.0)
      .child(
        DockLayout::new(bound.clone(), panels(&notes))
          .empty(Text::new("Open a file to start"))
          .mount(ctx),
      )
      .into()
  });
  stage.tree.resize(860, 440);
  stage.pass();
  assert!(stage.paints("Open a file to start"));
}

/// The scroll's offset: how far its first row stands above the scroll's top.
fn scrolled(stage: &mut Stage) -> f32 {
  let (_, top, ..) = stage.bounds("scroll");
  let (_, row, ..) = stage.bounds("row-0");
  top - row
}

/// Scrolls the explorer's list by 100 and puts the caret of its input after "ab": what a move must keep.
fn mark(stage: &mut Stage) {
  let (x, y, ..) = stage.bounds("scroll");
  stage.tree.scroll(x + 20.0, y + 20.0, 0.0, -100.0, ScrollPhase::Scroll);
  stage.pass();
  stage.pass();
  assert!(scrolled(stage) > 50.0, "scrolled: {}", scrolled(stage));
  stage.click("notes");
  stage.key("End");
  stage.key("ArrowLeft");
}

/// What a move kept: the scroll where it was, and, where nothing else was pressed, the focus on the input with its
/// caret where it was.
fn kept(stage: &mut Stage, notes: &Signal<String>, before: f32, focus: bool) {
  stage.pass();
  stage.pass();
  assert!(
    (scrolled(stage) - before).abs() < 1.0,
    "the scroll is kept: {} then {}",
    before,
    scrolled(stage)
  );
  if !focus {
    return;
  }
  assert_eq!(focused(stage).as_deref(), Some("notes"), "the focus is kept");
  stage.key("X");
  assert_eq!(notes.get_untracked(), "abXc", "the caret is kept");
  stage.key("Backspace");
}

#[test]
fn what_a_panel_holds_is_kept_when_it_moves_to_another_group() {
  let (state, notes, _, mut stage) = fresh();
  mark(&mut stage);
  let before = scrolled(&mut stage);
  let mut next = state.get_untracked();
  next.move_panel(
    "explorer",
    DockPlace::Tab {
      of: "terminal".to_owned(),
    },
  );
  next.select("explorer");
  state.set(next);
  stage.pass();
  kept(&mut stage, &notes, before, true);
}

#[test]
fn what_a_panel_holds_is_kept_when_it_floats_and_comes_back() {
  let (state, notes, _, mut stage) = fresh();
  mark(&mut stage);
  let before = scrolled(&mut stage);
  let mut next = state.get_untracked();
  next.move_panel(
    "explorer",
    DockPlace::Float {
      position: (300.0, 60.0),
      size: Some((260.0, 300.0)),
    },
  );
  state.set(next);
  stage.pass();
  kept(&mut stage, &notes, before, true);
  let mut next = state.get_untracked();
  next.move_panel("explorer", DockPlace::Edge(DockSide::Left));
  state.set(next);
  stage.pass();
  kept(&mut stage, &notes, before, true);
}

#[test]
fn what_a_panel_holds_is_kept_when_it_is_hidden_and_shown() {
  let (state, notes, _, mut stage) = fresh();
  mark(&mut stage);
  let before = scrolled(&mut stage);
  let mut next = state.get_untracked();
  next.move_panel("explorer", DockPlace::Hidden(DockSide::Left));
  state.set(next);
  stage.pass();
  assert!(stage.tree.get_element_by_id_mut("scroll").is_none(), "hidden");
  stage.click("dock-strip-explorer");
  kept(&mut stage, &notes, before, false);
}

#[test]
fn what_a_panel_holds_is_kept_when_its_group_is_maximized_and_a_split_is_dragged() {
  let (state, notes, told, mut stage) = fresh();
  mark(&mut stage);
  let before = scrolled(&mut stage);
  let mut next = state.get_untracked();
  next.maximize("explorer");
  state.set(next);
  stage.pass();
  kept(&mut stage, &notes, before, true);
  let mut next = state.get_untracked();
  next.restore();
  state.set(next);
  stage.pass();
  kept(&mut stage, &notes, before, true);
  // A drag of the split between the explorer and the rest: the shares change, what the panel holds stays.
  let (_, _, gw, _) = stage.bounds("dock-group-explorer");
  let at = stage.center("dock-split-root-divider-0");
  hold_to(&mut stage, at, (at.0 + 60.0, at.1));
  release(&mut stage, (at.0 + 60.0, at.1));
  let (_, _, wider, _) = stage.bounds("dock-group-explorer");
  assert!(wider > gw + 40.0, "{gw} then {wider}");
  assert!(!told.lock().unwrap().is_empty(), "the new shares are told");
  kept(&mut stage, &notes, before, true);
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_the_layout_its_groups_its_tabs_and_its_strip() {
  let (_, _, _, mut stage) = fresh();
  let describe = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(describe(&mut stage, "dock", "a11y_role").as_deref(), Some("group"));
  assert_eq!(
    describe(&mut stage, "dock", "a11y_name").as_deref(),
    Some("Dock layout")
  );
  assert_eq!(
    describe(&mut stage, "dock-tab-terminal", "a11y_role").as_deref(),
    Some("tab")
  );
  assert_eq!(
    describe(&mut stage, "dock-tab-terminal", "a11y_selected").as_deref(),
    Some("true")
  );
  assert_eq!(
    describe(&mut stage, "dock-strip-outline", "a11y_name").as_deref(),
    Some("Outline")
  );
  assert_eq!(
    describe(&mut stage, "dock-group-explorer", "a11y_role").as_deref(),
    Some("group")
  );
}

#[test]
fn a_theme_restyles_its_parts() {
  let state = Signal::new(board());
  let notes = Signal::new(String::new());
  let bound = state.clone();
  let mut stage = Stage::new(move |ctx: &mut Ctx| -> Element {
    Column::new()
      .size(860.0, 440.0)
      .child(
        DockLayout::new(bound.clone(), panels(&notes))
          .id("dock")
          .style(DockLayoutPart::Tab, |s| s.height(40.0))
          .mount(ctx),
      )
      .into()
  });
  stage.tree.resize(860, 440);
  stage.pass();
  assert_eq!(stage.bounds("dock-tab-terminal").3, 40.0);
}

#[test]
fn a_title_longer_than_its_room_is_cut() {
  let state = Signal::new(DockState::new(
    DockNode::row([DockNode::group(["explorer"]), DockNode::group(["terminal", "problems"])]).shares([0.15, 0.85]),
  ));
  let bound = state.clone();
  let mut stage = Stage::new(move |ctx: &mut Ctx| -> Element {
    let panels = vec![
      DockPanel::new("explorer", "An explorer with a title far longer than its room").child(Text::new("x")),
      DockPanel::new("terminal", "Terminal").child(Text::new("t")),
      DockPanel::new("problems", "Problems").child(Text::new("p")),
    ];
    Column::new()
      .size(860.0, 440.0)
      .child(DockLayout::new(bound.clone(), panels).id("dock").mount(ctx))
      .into()
  });
  stage.tree.resize(860, 440);
  stage.pass();
  let (gx, _, gw, _) = stage.bounds("dock-group-explorer");
  let painted = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text.starts_with("AN EXPLORER")))
    .map(|quad| quad.x + quad.width)
    .fold(0.0_f32, f32::max);
  assert!(
    painted <= gx + gw + 1.0,
    "the title stays in its group: {painted} > {}",
    gx + gw
  );
}

#[test]
fn a_floating_group_dragged_by_its_title_bar_onto_a_zone_docks_there() {
  let mut first = board();
  first.move_panel(
    "output",
    DockPlace::Float {
      position: (520.0, 60.0),
      size: Some((240.0, 160.0)),
    },
  );
  let state = Signal::new(first);
  let (notes, told) = (Signal::new(String::new()), Arc::new(Mutex::new(Vec::new())));
  let mut stage = scene(&state, &notes, &told);
  let (fx, fy, ..) = stage.bounds("dock-float-output");
  let middle = body_middle(&mut stage, "explorer", 30.0);
  hold_to(&mut stage, (fx + 80.0, fy + 15.0), middle);
  assert_eq!(
    zones(&stage, light(PaletteColor::Info)),
    1,
    "the indicator over the explorer"
  );
  release(&mut stage, middle);
  let now = state.get_untracked();
  assert!(!now.is_floating("output"));
  assert_eq!(
    now.group_of("output").map(|group| group.panels.clone()),
    Some(vec!["explorer".to_owned(), "output".to_owned()])
  );
}
