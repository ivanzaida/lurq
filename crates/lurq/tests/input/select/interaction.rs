use lurq::{
  app::{
    Tree,
    events::{MouseButton, ScrollPhase},
  },
  components::{Column, Rect, Select, Text},
  core::{ElementRef, Signal},
  node::{SelectPartStyle, SelectStyle, color::Color},
};

use crate::support::{pointer_click, render_pass, run_pass};

fn options() -> [(String, &'static str); 3] {
  [("sm", "Small"), ("md", "Medium"), ("lg", "Large")].map(|(v, l)| (v.to_owned(), l))
}

fn named_options(prefix: &str) -> [(String, String); 3] {
  [
    ("sm".to_owned(), format!("{prefix} Small")),
    ("md".to_owned(), format!("{prefix} Medium")),
    ("lg".to_owned(), format!("{prefix} Large")),
  ]
}

#[test]
fn single_select_opens_and_commits() {
  let value = Signal::new("md".to_owned());
  let mut tree = Tree::new();
  tree.set_root(Select::new(value.clone()).options(options()).width(200.0).height(40.0));
  run_pass(&mut tree);

  let trigger = tree
    .find_element(|el| el.tag_name() == "Select")
    .expect("trigger present");
  let bounds = trigger.bounds();
  tracing::debug!("trigger bounds: {bounds:?}");
  assert!(bounds.width > 0.0, "trigger laid out");

  let (tx, ty) = bounds.center();
  pointer_click(&mut tree, tx, ty, MouseButton::Left);
  run_pass(&mut tree);

  let large = tree.find_element(|el| el.text_content() == Some("Large"));
  tracing::debug!("'Large' option present after open: {}", large.is_some());
  assert!(large.is_some(), "menu options render when open");

  let large_bounds = large.unwrap().bounds();
  tracing::debug!("'Large' bounds: {large_bounds:?}");
  let (lx, ly) = large_bounds.center();
  pointer_click(&mut tree, lx, ly, MouseButton::Left);
  run_pass(&mut tree);

  tracing::debug!("value after click: {}", value.get());
  assert_eq!(value.get(), "lg", "clicking an option commits its value");
}

#[test]
fn custom_trigger_slot_renders_selected_state() {
  let value = Signal::new("md".to_owned());
  let mut tree = Tree::new();
  tree.set_root(
    Select::new(value)
      .options(options())
      .trigger(|state| Text::new(&format!("slot: {}", state.label.unwrap_or_default())))
      .width(200.0)
      .height(40.0),
  );
  run_pass(&mut tree);

  assert!(
    tree
      .find_element(|el| el.text_content() == Some("slot: Medium"))
      .is_some(),
    "custom trigger slot should receive and render the selected label"
  );
}

#[test]
fn clicking_outside_blurs_focused_select_and_closes_menu() {
  let value = Signal::new("md".to_owned());
  let select_ref = ElementRef::new();
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .spacing(12.0)
      .child(
        Select::new(value)
          .options(options())
          .width(200.0)
          .height(40.0)
          .ref_element(select_ref.clone()),
      )
      .child(Rect::new(200.0, 40.0).background("#ef4444")),
  );
  run_pass(&mut tree);

  let (x, y) = select_ref.bounds().center();
  pointer_click(&mut tree, x, y, MouseButton::Left);
  run_pass(&mut tree);

  assert!(select_ref.focused());
  assert!(tree.find_element(|el| el.text_content() == Some("Large")).is_some());

  pointer_click(&mut tree, 500.0, 500.0, MouseButton::Left);
  run_pass(&mut tree);

  assert!(!select_ref.focused());
  assert!(tree.find_element(|el| el.text_content() == Some("Large")).is_none());
}

#[test]
fn opening_another_select_blurs_previous_select() {
  let first_ref = ElementRef::new();
  let second_ref = ElementRef::new();
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .spacing(180.0)
      .child(
        Select::new(Signal::new("md".to_owned()))
          .options(named_options("First"))
          .width(200.0)
          .height(40.0)
          .ref_element(first_ref.clone()),
      )
      .child(
        Select::new(Signal::new("sm".to_owned()))
          .options(named_options("Second"))
          .width(200.0)
          .height(40.0)
          .ref_element(second_ref.clone()),
      ),
  );
  run_pass(&mut tree);

  let (x, y) = first_ref.bounds().center();
  pointer_click(&mut tree, x, y, MouseButton::Left);
  run_pass(&mut tree);
  assert!(first_ref.focused());
  assert!(
    tree
      .find_element(|el| el.text_content() == Some("First Large"))
      .is_some(),
    "first menu should open"
  );

  let (x, y) = second_ref.bounds().center();
  pointer_click(&mut tree, x, y, MouseButton::Left);
  run_pass(&mut tree);

  assert!(!first_ref.focused());
  assert!(second_ref.focused());
  assert!(
    tree
      .find_element(|el| el.text_content() == Some("First Large"))
      .is_none(),
    "opening the second select should close the first menu"
  );
  assert!(
    tree
      .find_element(|el| el.text_content() == Some("Second Large"))
      .is_some(),
    "second menu should stay open"
  );
}

#[test]
fn single_select_commits_when_redrawn_between_option_down_and_up() {
  let value = Signal::new("md".to_owned());
  let mut tree = Tree::new();
  tree.set_root(Select::new(value.clone()).options(options()).width(200.0).height(40.0));
  run_pass(&mut tree);

  let bounds = tree
    .find_element(|el| el.tag_name() == "Select")
    .expect("trigger present")
    .bounds();
  let (tx, ty) = bounds.center();
  pointer_click(&mut tree, tx, ty, MouseButton::Left);
  run_pass(&mut tree);

  let large = tree
    .find_element(|el| el.text_content() == Some("Large"))
    .expect("Large option");
  let (lx, ly) = large.bounds().center();
  tree.mouse_down(lx, ly, MouseButton::Left);
  run_pass(&mut tree);
  tree.mouse_up(lx, ly, MouseButton::Left);
  run_pass(&mut tree);

  assert_eq!(
    value.get(),
    "lg",
    "option clicks still commit when the menu redraws between press and release"
  );
}

#[test]
fn single_select_commits_on_option_mouse_down() {
  let value = Signal::new("md".to_owned());
  let mut tree = Tree::new();
  tree.set_root(Select::new(value.clone()).options(options()).width(200.0).height(40.0));
  run_pass(&mut tree);

  let bounds = tree
    .find_element(|el| el.tag_name() == "Select")
    .expect("trigger present")
    .bounds();
  let (tx, ty) = bounds.center();
  pointer_click(&mut tree, tx, ty, MouseButton::Left);
  run_pass(&mut tree);

  let large = tree
    .find_element(|el| el.text_content() == Some("Large"))
    .expect("Large option");
  let (lx, ly) = large.bounds().center();
  tree.mouse_down(lx, ly, MouseButton::Left);
  run_pass(&mut tree);

  assert_eq!(
    value.get(),
    "lg",
    "pressing an option should commit even if the following mouse-up is missed"
  );
}

#[test]
fn select_inside_scroll_opens_and_commits() {
  use lurq::components::{Column, Rect, ScrollVertical};
  let value = Signal::new("md".to_owned());
  let mut tree = Tree::new();
  tree.set_root(
    ScrollVertical::new(
      Column::new()
        .child(Select::new(value.clone()).options(options()).width(200.0).height(40.0))
        .child(Rect::new(200.0, 2000.0)),
    )
    .width(400.0)
    .height(600.0),
  );
  run_pass(&mut tree);

  let bounds = tree
    .find_element(|el| el.tag_name() == "Select")
    .expect("trigger present")
    .bounds();
  tracing::debug!("scroll trigger bounds: {bounds:?}");
  let (tx, ty) = bounds.center();
  pointer_click(&mut tree, tx, ty, MouseButton::Left);
  run_pass(&mut tree);

  let large = tree.find_element(|el| el.text_content() == Some("Large"));
  tracing::debug!("'Large' present (in scroll): {}", large.is_some());
  assert!(large.is_some(), "menu renders for select inside scroll");
  let lb = large.unwrap().bounds();
  tracing::debug!("'Large' bounds (in scroll): {lb:?}");
  let (lx, ly) = lb.center();
  pointer_click(&mut tree, lx, ly, MouseButton::Left);
  run_pass(&mut tree);
  tracing::debug!("scroll value after click: {}", value.get());
  assert_eq!(value.get(), "lg", "commit works for select inside scroll");
}

#[test]
fn scrolled_select_menu_commits_clicked_option() {
  let value = Signal::new("item-00".to_owned());
  let options = (0..20)
    .map(|index| (format!("item-{index:02}"), format!("Option {index:02}")))
    .collect::<Vec<_>>();
  let mut tree = Tree::new();
  tree.set_root(
    Select::new(value.clone())
      .options(options)
      .width(200.0)
      .height(40.0)
      .style(SelectStyle::new().max_menu_height(120.0)),
  );
  run_pass(&mut tree);

  let trigger = tree
    .find_element(|el| el.tag_name() == "Select")
    .expect("trigger present");
  let (tx, ty) = trigger.bounds().center();
  pointer_click(&mut tree, tx, ty, MouseButton::Left);
  run_pass(&mut tree);

  tree.scroll(tx, ty + 70.0, 0.0, -900.0, ScrollPhase::Scroll);
  run_pass(&mut tree);

  let option = tree
    .find_element(|el| el.text_content() == Some("Option 19"))
    .expect("last option should be laid out after scrolling menu");
  let (ox, oy) = option.bounds().center();
  pointer_click(&mut tree, ox, oy, MouseButton::Left);
  run_pass(&mut tree);

  assert_eq!(
    value.get(),
    "item-19",
    "clicking an option after scrolling the select menu commits its value"
  );
}

#[test]
fn multi_select_toggles_values() {
  let value: Signal<Vec<String>> = Signal::new(Vec::new());
  let mut tree = Tree::new();
  tree.set_root(
    Select::multiple(value.clone())
      .options(options())
      .width(200.0)
      .height(40.0),
  );
  run_pass(&mut tree);

  let bounds = tree
    .find_element(|el| el.tag_name() == "Select")
    .expect("trigger present")
    .bounds();
  let (tx, ty) = bounds.center();
  pointer_click(&mut tree, tx, ty, MouseButton::Left);
  run_pass(&mut tree);

  let small = tree
    .find_element(|el| el.text_content() == Some("Small"))
    .expect("Small option");
  let (sx, sy) = small.bounds().center();
  pointer_click(&mut tree, sx, sy, MouseButton::Left);
  run_pass(&mut tree);

  tracing::debug!("multi value after first click: {:?}", value.get());
  assert_eq!(value.get(), vec!["sm".to_owned()], "multi-select adds the value");

  // Menu must stay open for multi-select so a second option can be picked
  // without reopening.
  let large = tree.find_element(|el| el.text_content() == Some("Large"));
  tracing::debug!("menu still open after first multi pick: {}", large.is_some());
  assert!(large.is_some(), "multi-select menu stays open after a pick");
  let (lx, ly) = large.unwrap().bounds().center();
  pointer_click(&mut tree, lx, ly, MouseButton::Left);
  run_pass(&mut tree);
  tracing::debug!("multi value after second click: {:?}", value.get());
  assert_eq!(
    value.get(),
    vec!["sm".to_owned(), "lg".to_owned()],
    "second multi pick adds without reopening"
  );
}

#[test]
fn multi_select_prefilled_rows_share_selected_background_on_open() {
  let value: Signal<Vec<String>> = Signal::new(vec!["sm".to_owned(), "md".to_owned(), "lg".to_owned()]);
  let selected = Color::from_hex("#0ea5e9");
  let highlighted = Color::from_hex("#1e293b");
  let mut tree = Tree::new();
  tree.set_root(
    Select::multiple(value)
      .options(options())
      .width(200.0)
      .height(40.0)
      .style(
        SelectStyle::new()
          .option_hovered(SelectPartStyle::new().background(highlighted))
          .option_selected(SelectPartStyle::new().background(selected)),
      ),
  );
  run_pass(&mut tree);

  let bounds = tree
    .find_element(|el| el.tag_name() == "Select")
    .expect("trigger present")
    .bounds();
  let (tx, ty) = bounds.center();
  pointer_click(&mut tree, tx, ty, MouseButton::Left);
  let snapshot = render_pass(&mut tree);

  let selected_rows = snapshot
    .rects
    .iter()
    .filter(|rect| rect.color == selected && rect.width >= 190.0 && rect.height > 20.0)
    .count();
  let highlighted_rows = snapshot
    .rects
    .iter()
    .filter(|rect| rect.color == highlighted && rect.width >= 190.0 && rect.height > 20.0)
    .count();

  assert_eq!(
    selected_rows, 3,
    "every prefilled option should use the selected background"
  );
  assert_eq!(
    highlighted_rows, 0,
    "opening the menu should not paint the prefilled row as hovered"
  );
}

#[test]
fn multi_select_mouse_open_does_not_highlight_first_unselected_row() {
  let value: Signal<Vec<String>> = Signal::new(Vec::new());
  let highlighted = Color::from_hex("#1e293b");
  let mut tree = Tree::new();
  tree.set_root(
    Select::multiple(value)
      .options(options())
      .width(200.0)
      .height(40.0)
      .style(SelectStyle::new().option_hovered(SelectPartStyle::new().background(highlighted))),
  );
  run_pass(&mut tree);

  let bounds = tree
    .find_element(|el| el.tag_name() == "Select")
    .expect("trigger present")
    .bounds();
  let (tx, ty) = bounds.center();
  pointer_click(&mut tree, tx, ty, MouseButton::Left);
  let snapshot = render_pass(&mut tree);

  let highlighted_rows = snapshot
    .rects
    .iter()
    .filter(|rect| rect.color == highlighted && rect.width >= 190.0 && rect.height > 20.0)
    .count();

  assert_eq!(
    highlighted_rows, 0,
    "mouse opening an empty multi-select should not highlight row 0"
  );
}

#[test]
fn selected_edge_options_inherit_menu_corner_radius() {
  let selected = Color::from_hex("#0ea5e9");
  for (value, expected_radii) in [("sm", [6.0, 6.0, 0.0, 0.0]), ("lg", [0.0, 0.0, 6.0, 6.0])] {
    let value = Signal::new(value.to_owned());
    let mut tree = Tree::new();
    tree.set_root(
      Select::new(value).options(options()).width(200.0).height(40.0).style(
        SelectStyle::new()
          .menu(SelectPartStyle::new().rounded(6.0))
          .option_selected(SelectPartStyle::new().background(selected)),
      ),
    );
    run_pass(&mut tree);

    let bounds = tree
      .find_element(|el| el.tag_name() == "Select")
      .expect("trigger present")
      .bounds();
    let (tx, ty) = bounds.center();
    pointer_click(&mut tree, tx, ty, MouseButton::Left);
    let snapshot = render_pass(&mut tree);

    let selected_row = snapshot
      .rects
      .iter()
      .find(|rect| rect.color == selected && rect.width >= 190.0 && rect.height > 20.0)
      .expect("selected option row should render");
    assert_eq!(selected_row.radii, expected_radii);
  }
}

#[test]
fn overflowing_option_stays_ellipsized_on_hover() {
  let long_label = "Speakers (High Definition Audio Device With Extra Long Name)";
  let value = Signal::new("md".to_owned());
  let mut tree = Tree::new();
  tree.set_root(
    Select::new(value)
      .options([("md".to_owned(), "Medium"), ("long".to_owned(), long_label)])
      .width(200.0)
      .height(40.0),
  );
  run_pass(&mut tree);

  let trigger = tree
    .find_element(|el| el.tag_name() == "Select")
    .expect("trigger present");
  let (x, y) = trigger.bounds().center();
  pointer_click(&mut tree, x, y, MouseButton::Left);
  let before_hover = render_pass(&mut tree);

  let option = tree
    .find_element(|el| el.text_content() == Some(long_label))
    .expect("long option present");
  let (x, y) = option.bounds().center();
  tree.mouse_move(x, y);
  let after_hover = render_pass(&mut tree);

  assert_eq!(
    after_hover.glyph_count, before_hover.glyph_count,
    "hovering an overflowing option should not briefly render the full label"
  );
}
