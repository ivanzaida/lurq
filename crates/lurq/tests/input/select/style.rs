use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use lurq::{
  app::{App, Tree, events::MouseButton},
  components::{Column, Rect, Select, SelectOption, Text},
  core::{ElementRef, Signal},
  node::{
    BoxShadow, Element, SelectCheckmarkPosition, SelectIcon, SelectPartStyle, SelectStyle, color::Color,
    padding::Padding,
  },
};

use crate::support::{RenderSnapshot, pointer_click, render_pass_with_app};

const SELECT_ID: &str = "select";

fn options() -> Vec<SelectOption<String>> {
  vec![
    SelectOption::new("sm".to_owned(), "Small"),
    SelectOption::new("md".to_owned(), "Medium"),
    SelectOption::new("lg".to_owned(), "Large"),
  ]
}

struct Fixture {
  tree: Tree,
  app: App,
  select: ElementRef,
}

impl Fixture {
  /// A select with the default trigger, which shows the chevron.
  fn new(value: &str, style: SelectStyle) -> Self {
    Self::build(value, options(), style, false)
  }

  /// A select whose trigger does not repeat the selected label, so option
  /// labels are unique in the tree.
  fn with_options(value: &str, options: Vec<SelectOption<String>>, style: SelectStyle) -> Self {
    Self::build(value, options, style, true)
  }

  fn build(value: &str, options: Vec<SelectOption<String>>, style: SelectStyle, plain_trigger: bool) -> Self {
    let select = ElementRef::new();
    let mut tree = Tree::new();
    let mut control = Select::new(Signal::new(value.to_owned()))
      .options(options)
      .width(200.0)
      .height(40.0)
      .style(style)
      .ref_element(select.clone());
    if plain_trigger {
      control = control.trigger(|_| Text::new("Pick"));
    }
    tree.set_root(Element::from(control).id(SELECT_ID));
    let mut app = App::new();
    render_pass_with_app(&mut tree, &mut app);
    Self { tree, app, select }
  }

  fn render(&mut self) -> RenderSnapshot {
    render_pass_with_app(&mut self.tree, &mut self.app)
  }

  fn pointer_open(&mut self) -> RenderSnapshot {
    let (x, y) = self.select.bounds().center();
    pointer_click(&mut self.tree, x, y, MouseButton::Left);
    self.render()
  }

  fn keyboard_open(&mut self) -> RenderSnapshot {
    self
      .tree
      .get_element_by_id_mut(SELECT_ID)
      .expect("select has an id")
      .focus();
    self
      .tree
      .key_down("Enter".to_owned(), "Enter".to_owned(), false, false, false);
    self.render()
  }

  fn bounds_of(&mut self, text: &str) -> lurq::core::ElementRect {
    self
      .tree
      .find_element(|el| el.text_content() == Some(text))
      .unwrap_or_else(|| panic!("{text} is laid out"))
      .bounds()
  }

  fn menu_bounds(&mut self) -> lurq::core::ElementRect {
    self
      .tree
      .find_element(|el| el.tag_name() == "SelectMenu")
      .expect("menu open")
      .bounds()
  }

  fn has_text(&mut self, text: &str) -> bool {
    self.tree.find_element(|el| el.text_content() == Some(text)).is_some()
  }
}

fn glyphs_colored(snapshot: &RenderSnapshot, color: &str) -> usize {
  let expected = Color::from_hex(color).to_linear_f32_array();
  snapshot
    .glyphs
    .iter()
    .filter(|glyph| {
      glyph
        .color
        .iter()
        .zip(expected.iter())
        .all(|(actual, expected)| (actual - expected).abs() < 0.01)
    })
    .count()
}

#[test]
fn highlight_part_composes_with_selected_and_hover() {
  let fill = Color::from_hex("#334155");
  let ring = BoxShadow::new(0.0, 0.0, 0.0, Color::from_hex("#38bdf8"))
    .spread(2.0)
    .inset();
  let style = SelectStyle::new()
    .option_hovered(SelectPartStyle::new().background(fill))
    .option_selected(SelectPartStyle::new())
    .option_selected_hovered(SelectPartStyle::new().background(fill))
    .option_highlighted(SelectPartStyle::new().box_shadow(ring))
    .single_checkmark(true);
  let mut fixture = Fixture::with_options("md", options(), style);
  let snapshot = fixture.keyboard_open();

  let rings: Vec<_> = snapshot
    .rects
    .iter()
    .filter(|rect| rect.shadow.is_some_and(|shadow| shadow.inset) && rect.width >= 150.0)
    .collect();
  assert_eq!(rings.len(), 1, "the highlighted selected option draws its ring");
  assert!(
    !snapshot.rects.iter().any(|rect| rect.color == fill),
    "the keyboard highlight does not borrow the hover fill once option_highlighted is set"
  );
  assert!(fixture.has_text("\u{2713}"), "the selected option keeps its checkmark");

  let medium = fixture.bounds_of("Medium");
  fixture.tree.mouse_move(medium.x + 20.0, medium.y + 5.0);
  let hovered = fixture.render();
  assert!(
    hovered
      .rects
      .iter()
      .any(|rect| rect.color == fill && rect.width >= 150.0),
    "the pointer hover adds its fill to the highlighted row"
  );
  assert!(
    hovered
      .rects
      .iter()
      .any(|rect| rect.shadow.is_some_and(|shadow| shadow.inset)),
    "and the ring stays"
  );
}

#[test]
fn single_select_checkmark_is_opt_in() {
  let mut fixture = Fixture::new("md", SelectStyle::new());
  fixture.pointer_open();
  assert!(!fixture.has_text("\u{2713}"), "hidden by default");

  let mut fixture = Fixture::new("md", SelectStyle::new().single_checkmark(true));
  fixture.pointer_open();
  assert!(fixture.has_text("\u{2713}"), "shown on the selected option");
}

#[test]
fn checkmark_slot_is_reserved_on_every_option() {
  let style = SelectStyle::new().single_checkmark(true).checkmark_size(14.0);
  let mut fixture = Fixture::with_options("md", options(), style);
  fixture.pointer_open();
  let small = fixture.bounds_of("Small");
  let medium = fixture.bounds_of("Medium");
  assert_eq!(
    small.x, medium.x,
    "labels line up whether or not the option is selected"
  );
  let check = fixture.bounds_of("\u{2713}");
  assert!(check.x < medium.x, "leading by default");
}

#[test]
fn trailing_checkmark_sits_after_the_label() {
  let style = SelectStyle::new()
    .single_checkmark(true)
    .checkmark_size(14.0)
    .checkmark_position(SelectCheckmarkPosition::Trailing)
    .checkmark(SelectIcon::text("*"));
  let mut fixture = Fixture::with_options("md", options(), style);
  fixture.pointer_open();
  let menu = fixture.menu_bounds();
  let medium = fixture.bounds_of("Medium");
  let check = fixture.bounds_of("*");
  assert!(check.x > medium.x + medium.width - 1.0, "after the label");
  assert!(check.x + check.width <= menu.x + menu.width, "inside the row");
  let small = fixture.bounds_of("Small");
  assert_eq!(small.x, medium.x);
}

#[test]
fn open_chevron_replaces_the_closed_one_while_open() {
  let style = SelectStyle::new()
    .chevron(SelectIcon::text("v"))
    .chevron_open(SelectIcon::text("^"))
    .chevron_color(Color::from_hex("#ef4444"))
    .trigger_open(SelectPartStyle::new().background(Color::from_hex("#0f172a")));
  let mut fixture = Fixture::new("md", style.clone());
  let closed = fixture.render();
  assert_eq!(glyphs_colored(&closed, "#ef4444"), 1, "only one chevron paints");

  let mut open_fixture = Fixture::new("md", style.chevron_color(Color::from_hex("#22c55e")));
  let open = open_fixture.pointer_open();
  assert_eq!(
    glyphs_colored(&open, "#22c55e"),
    1,
    "only one chevron paints while open"
  );
  let chevron = open_fixture.bounds_of("^");
  assert!(chevron.width > 0.0);
}

#[test]
fn chevron_swap_follows_the_open_state() {
  let open_color = "#22c55e";
  let closed_color = "#ef4444";
  let style = SelectStyle::new()
    .chevron(SelectIcon::element(move |_| {
      Text::new("v").color(Color::from_hex(closed_color)).into()
    }))
    .chevron_open(SelectIcon::element(move |_| {
      Text::new("^").color(Color::from_hex(open_color)).into()
    }));
  let mut fixture = Fixture::new("md", style);
  let closed = fixture.render();
  assert_eq!(glyphs_colored(&closed, closed_color), 1);
  assert_eq!(glyphs_colored(&closed, open_color), 0);
  let open = fixture.pointer_open();
  assert_eq!(glyphs_colored(&open, closed_color), 0);
  assert_eq!(glyphs_colored(&open, open_color), 1);
}

#[test]
fn element_icon_receives_the_configured_color() {
  let received = Arc::new(AtomicBool::new(false));
  let seen = received.clone();
  let style = SelectStyle::new()
    .chevron_color(Color::from_hex("#ef4444"))
    .chevron(SelectIcon::element(move |color| {
      seen.store(color.is_some(), Ordering::Relaxed);
      Rect::new(8.0, 8.0).into()
    }));
  let _fixture = Fixture::new("md", style);
  assert!(received.load(Ordering::Relaxed));
}

#[test]
fn menu_padding_insets_options_and_menu_shadow_paints() {
  let style = SelectStyle::new()
    .menu(
      SelectPartStyle::new()
        .background(Color::from_hex("#111827"))
        .padding(Padding::all(4.0))
        .box_shadow(BoxShadow::new(0.0, 8.0, 24.0, Color::from_hex("#00000066"))),
    )
    .option(
      SelectPartStyle::new()
        .min_height(32.0)
        .padding(Padding::symmetric(8.0, 0.0))
        .rounded(7.0),
    );
  let mut fixture = Fixture::with_options("md", options(), style);
  let snapshot = fixture.pointer_open();
  let menu = fixture.menu_bounds();
  let small = fixture.bounds_of("Small");
  assert_eq!(small.x, menu.x + 4.0 + 8.0, "menu padding plus option padding");
  assert_eq!(menu.height, 3.0 * 32.0 + 8.0, "rows plus vertical menu padding");
  assert!(
    snapshot
      .rects
      .iter()
      .any(|rect| rect.shadow.is_some_and(|shadow| !shadow.inset)),
    "the menu shadow paints"
  );
}

#[test]
fn trigger_part_shadow_paints() {
  let style = SelectStyle::new().trigger_focused(
    SelectPartStyle::new().box_shadow(BoxShadow::new(0.0, 0.0, 0.0, Color::from_hex("#38bdf8")).spread(2.0)),
  );
  let mut fixture = Fixture::new("md", style);
  let closed = fixture.render();
  assert!(!closed.rects.iter().any(|rect| rect.shadow.is_some()));
  let focused = fixture.keyboard_open();
  assert!(focused.rects.iter().any(|rect| rect.shadow.is_some()));
}

#[test]
fn menu_flips_above_the_trigger_without_room_below() {
  let select = ElementRef::new();
  let mut tree = Tree::new();
  tree.resize(400, 300);
  tree.set_root(
    Column::new().child(Rect::new(200.0, 240.0)).child(
      Select::new(Signal::new("md".to_owned()))
        .options(options())
        .width(200.0)
        .height(40.0)
        .ref_element(select.clone()),
    ),
  );
  let mut app = App::new();
  render_pass_with_app(&mut tree, &mut app);
  let trigger = select.bounds();
  pointer_click(&mut tree, trigger.x + 20.0, trigger.y + 20.0, MouseButton::Left);
  render_pass_with_app(&mut tree, &mut app);
  let menu = tree
    .find_element(|el| el.tag_name() == "SelectMenu")
    .expect("menu open")
    .bounds();
  assert!(
    menu.y + menu.height <= trigger.y + 0.5,
    "menu {menu:?} should sit above trigger {trigger:?}"
  );
}

#[test]
fn option_detail_renders_under_the_label() {
  let options = vec![
    SelectOption::new("sm".to_owned(), "Small"),
    SelectOption::new("xl".to_owned(), "Extra large")
      .detail("Needs a bigger plan")
      .disabled(true),
  ];
  let mut fixture = Fixture::with_options("sm", options, SelectStyle::new());
  fixture.pointer_open();
  let label = fixture.bounds_of("Extra large");
  let detail = fixture.bounds_of("Needs a bigger plan");
  assert!(
    detail.y >= label.y + label.height - 0.5,
    "the detail line is below the label"
  );
  assert_eq!(detail.x, label.x);
}
