use lurq::{
  app::{App, Tree, events::MouseButton},
  components::{Select, SelectOption, Text},
  core::{ElementRef, Signal},
  node::{Element, SelectPartStyle, SelectStyle, color::Color},
};

use crate::support::{RenderSnapshot, pointer_click, render_pass_if_needed, render_pass_with_app, run_pass};

const MENU_TOP: f32 = 44.0;
const ROW_HEIGHT: f32 = 34.0;
const SELECT_ID: &str = "select";

fn hover() -> Color {
  Color::from_hex("#1e293b")
}

fn selected() -> Color {
  Color::from_hex("#0ea5e9")
}

fn selected_hover() -> Color {
  Color::from_hex("#0369a1")
}

fn style() -> SelectStyle {
  SelectStyle::new()
    .option_hovered(SelectPartStyle::new().background(hover()))
    .option_selected(SelectPartStyle::new().background(selected()))
    .option_selected_hovered(SelectPartStyle::new().background(selected_hover()))
}

fn options() -> Vec<SelectOption<String>> {
  vec![
    SelectOption::new("sm".to_owned(), "Small"),
    SelectOption::new("md".to_owned(), "Medium"),
    SelectOption::new("lg".to_owned(), "Large"),
    SelectOption::new("xl".to_owned(), "Extra large"),
  ]
}

fn options_with_disabled() -> Vec<SelectOption<String>> {
  vec![
    SelectOption::new("off".to_owned(), "Off").disabled(true),
    SelectOption::new("sm".to_owned(), "Small"),
    SelectOption::new("md".to_owned(), "Medium").disabled(true),
    SelectOption::new("lg".to_owned(), "Large"),
    SelectOption::new("xl".to_owned(), "Extra").disabled(true),
  ]
}

fn focus(tree: &mut Tree) {
  tree.get_element_by_id_mut(SELECT_ID).expect("select has an id").focus();
}

fn press(tree: &mut Tree, key: &str) {
  tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
}

struct Fixture {
  tree: Tree,
  app: App,
  value: Signal<String>,
  select: ElementRef,
}

impl Fixture {
  fn new(value: &str, options: Vec<SelectOption<String>>) -> Self {
    let value = Signal::new(value.to_owned());
    let select = ElementRef::new();
    let mut tree = Tree::new();
    tree.set_root(
      Element::from(
        Select::new(value.clone())
          .options(options)
          .width(200.0)
          .height(40.0)
          .style(style())
          .ref_element(select.clone()),
      )
      .id(SELECT_ID),
    );
    let mut app = App::new();
    render_pass_with_app(&mut tree, &mut app);
    Self {
      tree,
      app,
      value,
      select,
    }
  }

  fn pointer_open(&mut self) {
    let (x, y) = self.select.bounds().center();
    pointer_click(&mut self.tree, x, y, MouseButton::Left);
    render_pass_with_app(&mut self.tree, &mut self.app);
  }

  fn keyboard_open(&mut self) {
    focus(&mut self.tree);
    self.key("ArrowDown");
  }

  /// Presses `key` and runs passes only as far as the tree asks for them, so
  /// a key that forgets to invalidate the menu leaves no fresh frame.
  fn key(&mut self, key: &str) -> Option<RenderSnapshot> {
    press(&mut self.tree, key);
    let first = render_pass_if_needed(&mut self.tree, &mut self.app);
    render_pass_if_needed(&mut self.tree, &mut self.app).or(first)
  }

  fn highlighted_after(&mut self, key: &str) -> Option<usize> {
    let snapshot = self.key(key).expect("a key that moves the highlight redraws");
    highlighted_row(&snapshot)
  }

  fn highlighted_now(&mut self) -> Option<usize> {
    highlighted_row(&render_pass_with_app(&mut self.tree, &mut self.app))
  }

  fn menu_open(&mut self) -> bool {
    self.tree.find_element(|el| el.tag_name() == "SelectMenu").is_some()
  }
}

/// The row painted with a hover colour: the keyboard highlight's look when no
/// `option_highlighted` part is set.
fn highlighted_row(snapshot: &RenderSnapshot) -> Option<usize> {
  snapshot
    .rects
    .iter()
    .find(|rect| (rect.color == hover() || rect.color == selected_hover()) && rect.width >= 190.0)
    .map(|rect| ((rect.y - MENU_TOP) / ROW_HEIGHT).round() as usize)
}

#[test]
fn pointer_open_starts_without_highlight_and_first_arrow_lands_on_selected() {
  let mut fixture = Fixture::new("lg", options());
  fixture.pointer_open();
  assert_eq!(fixture.highlighted_now(), None, "a pointer open highlights nothing");

  assert_eq!(
    fixture.highlighted_after("ArrowDown"),
    Some(2),
    "the first ArrowDown highlights the selected option, drawn over its selected fill"
  );
  assert_eq!(fixture.highlighted_after("ArrowDown"), Some(3), "then moves from there");
}

#[test]
fn pointer_open_without_selection_first_arrow_lands_on_first_enabled() {
  let mut fixture = Fixture::new("none", options_with_disabled());
  fixture.pointer_open();
  assert_eq!(
    fixture.highlighted_after("ArrowDown"),
    Some(1),
    "skips the disabled first option"
  );
}

#[test]
fn arrows_skip_disabled_options_and_stop_at_the_ends() {
  let mut fixture = Fixture::new("sm", options_with_disabled());
  fixture.keyboard_open();
  assert_eq!(fixture.highlighted_now(), Some(1));
  assert_eq!(fixture.highlighted_after("ArrowDown"), Some(3), "skips disabled Medium");
  fixture.key("ArrowDown");
  assert_eq!(fixture.highlighted_now(), Some(3), "stays on the last enabled option");
  assert_eq!(fixture.highlighted_after("ArrowUp"), Some(1));
  fixture.key("ArrowUp");
  assert_eq!(
    fixture.highlighted_now(),
    Some(1),
    "does not wrap past the first enabled option"
  );
}

#[test]
fn home_and_end_jump_to_enabled_edges() {
  let mut fixture = Fixture::new("sm", options_with_disabled());
  fixture.keyboard_open();
  assert_eq!(fixture.highlighted_after("End"), Some(3));
  assert_eq!(fixture.highlighted_after("Home"), Some(1));
}

#[test]
fn keyboard_open_highlights_selected_option() {
  for key in ["Enter", " ", "ArrowDown", "ArrowUp"] {
    let mut fixture = Fixture::new("md", options());
    focus(&mut fixture.tree);
    let snapshot = fixture.key(key).expect("opening redraws");
    assert!(fixture.menu_open(), "{key:?} opens the menu");
    assert_eq!(
      highlighted_row(&snapshot),
      Some(1),
      "{key:?} highlights the selected option"
    );
  }
}

#[test]
fn alt_arrow_down_opens() {
  let mut fixture = Fixture::new("md", options());
  focus(&mut fixture.tree);
  fixture
    .tree
    .key_down("ArrowDown".to_owned(), "ArrowDown".to_owned(), false, false, true);
  render_pass_with_app(&mut fixture.tree, &mut fixture.app);
  assert!(fixture.menu_open());
}

#[test]
fn enter_and_space_commit_the_highlighted_option_and_close() {
  for key in ["Enter", " "] {
    let mut fixture = Fixture::new("sm", options());
    fixture.keyboard_open();
    fixture.key("ArrowDown");
    fixture.key(key);
    assert_eq!(fixture.value.get(), "md", "{key:?} commits the highlighted option");
    assert!(!fixture.menu_open(), "{key:?} closes a single-select");
    assert!(fixture.select.focused(), "focus stays on the trigger");
  }
}

#[test]
fn enter_without_highlight_closes_without_change() {
  let mut fixture = Fixture::new("lg", options());
  fixture.pointer_open();
  fixture.key("Enter");
  assert_eq!(fixture.value.get(), "lg");
  assert!(!fixture.menu_open());
}

#[test]
fn escape_and_tab_close_without_change_and_keep_focus() {
  for key in ["Escape", "Tab"] {
    let mut fixture = Fixture::new("sm", options());
    fixture.keyboard_open();
    fixture.key("ArrowDown");
    fixture.key(key);
    assert_eq!(fixture.value.get(), "sm", "{key} discards the highlight");
    assert!(!fixture.menu_open(), "{key} closes the menu");
    assert!(fixture.select.focused(), "{key} leaves focus on the trigger");
  }
}

#[test]
fn typing_highlights_the_next_enabled_option_with_that_prefix() {
  let mut fixture = Fixture::new("sm", options());
  fixture.keyboard_open();
  assert_eq!(fixture.highlighted_after("e"), Some(3), "jumps to Extra large");

  let mut fixture = Fixture::new("sm", options_with_disabled());
  fixture.keyboard_open();
  fixture.key("m");
  assert_eq!(fixture.highlighted_now(), Some(1), "disabled Medium is not a match");
}

#[test]
fn repeating_a_letter_cycles_through_matches() {
  let options = vec![
    SelectOption::new("a".to_owned(), "Alpha"),
    SelectOption::new("b".to_owned(), "Beta"),
    SelectOption::new("c".to_owned(), "Also"),
  ];
  let mut fixture = Fixture::new("a", options);
  fixture.keyboard_open();
  assert_eq!(
    fixture.highlighted_after("a"),
    Some(2),
    "from Alpha to the next A option"
  );
  assert_eq!(fixture.highlighted_after("a"), Some(0), "and around");
}

#[test]
fn disabled_option_ignores_clicks() {
  let mut fixture = Fixture::new("sm", options_with_disabled());
  fixture.pointer_open();
  let medium = fixture
    .tree
    .find_element(|el| el.text_content() == Some("Medium"))
    .expect("disabled option is listed");
  let (x, y) = medium.bounds().center();
  pointer_click(&mut fixture.tree, x, y, MouseButton::Left);
  run_pass(&mut fixture.tree);
  assert_eq!(fixture.value.get(), "sm");
  assert!(fixture.menu_open(), "the menu stays open");
}

fn long_select(value: &str) -> (Tree, App, ElementRef) {
  let options = (0..20)
    .map(|index| (format!("item-{index:02}"), format!("Option {index:02}")))
    .collect::<Vec<_>>();
  let select = ElementRef::new();
  let mut tree = Tree::new();
  tree.set_root(
    Element::from(
      Select::new(Signal::new(value.to_owned()))
        .options(options)
        .width(200.0)
        .height(40.0)
        .style(SelectStyle::new().max_menu_height(120.0))
        .trigger(|_| Text::new("Pick"))
        .ref_element(select.clone()),
    )
    .id(SELECT_ID),
  );
  let mut app = App::new();
  render_pass_with_app(&mut tree, &mut app);
  (tree, app, select)
}

fn assert_option_visible(tree: &mut Tree, label: &str) {
  let menu = tree
    .find_element(|el| el.tag_name() == "SelectMenu")
    .expect("menu open")
    .bounds();
  let option = tree
    .find_element(|el| el.text_content() == Some(label))
    .expect("option laid out")
    .bounds();
  assert!(
    option.y >= menu.y - 0.5 && option.y + option.height <= menu.y + menu.height + 0.5,
    "{label} at {option:?} should be inside the menu viewport {menu:?}"
  );
}

#[test]
fn highlight_moved_by_keys_is_scrolled_into_view() {
  let (mut tree, mut app, _) = long_select("item-00");
  focus(&mut tree);
  for key in ["ArrowDown", "End"] {
    press(&mut tree, key);
    render_pass_with_app(&mut tree, &mut app);
  }
  assert_option_visible(&mut tree, "Option 19");
  press(&mut tree, "Home");
  render_pass_with_app(&mut tree, &mut app);
  assert_option_visible(&mut tree, "Option 00");
}

#[test]
fn opening_scrolls_the_selected_option_into_view() {
  let (mut tree, mut app, select) = long_select("item-15");
  let (x, y) = select.bounds().center();
  pointer_click(&mut tree, x, y, MouseButton::Left);
  render_pass_with_app(&mut tree, &mut app);
  assert_option_visible(&mut tree, "Option 15");
}
