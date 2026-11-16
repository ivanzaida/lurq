//! A tree select: its trigger and its panel against the board, opening and choosing with the pointer and the keys,
//! where the focus goes on each way out, its forms of one, one or none and any number, the rows that only lead to
//! others, the clear control, what its handlers are told, a text longer than its room, tooling, and a theme's other
//! anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row, Text},
  core::Signal,
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{
    Button, Field, Interaction, Themed, TreeItem, TreeSelect, TreeSelectPart, TreeSelectRenderer, TreeSelectSize,
    TreeSelectView,
  },
};

use super::stage::Stage;

type Id = &'static str;

const CHEVRON_DOWN: &str = "\u{E019}";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn folder(id: Id) -> TreeItem<Id> {
  TreeItem::folder(id, id).id(id)
}

fn file(id: Id) -> TreeItem<Id> {
  TreeItem::file(id, id).id(id)
}

/// The board's tree: crates, with lurq (lib.rs, theme.rs) and demo (main.rs) in it; Cargo.toml.
fn files() -> Vec<TreeItem<Id>> {
  vec![
    folder("crates").items([
      folder("lurq").items([file("lib.rs"), file("theme.rs")]),
      folder("demo").item(file("main.rs")),
    ]),
    file("Cargo.toml"),
  ]
}

/// What the app was told, in order.
type Told = Arc<Mutex<Vec<String>>>;

fn said(told: &Told) -> Vec<String> {
  std::mem::take(&mut *told.lock().unwrap())
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// A stage of `select`, 8 in from the corner of a window with room for its panel, with a button after it.
fn staged(select: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  Stage::new(move |ctx| {
    // The button stands beside the select, clear of the panel that opens under it.
    Row::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .spacing(8.0)
      .align_items(lurq::layout::Alignment::Start)
      .child(Column::new().flex(1.0).min_width(0.0).child(select(ctx)))
      .child(Button::new("After").id("after"))
      .into()
  })
}

/// A mounted select of one or none bound to a signal, which tells `told` what its handlers hear.
fn bound(value: Option<Id>) -> (Stage, Signal<Option<Id>>, Told) {
  let value = Signal::new(value);
  let told: Told = Arc::default();
  let (held, heard) = (value.clone(), told.clone());
  let stage = staged(move |ctx| {
    let (change, open, dismiss) = (heard.clone(), heard.clone(), heard.clone());
    let seen = held.clone();
    TreeSelect::optional(held.clone(), files())
      .label("File")
      .placeholder("Select a file")
      .id("select")
      .on_change(move |value| {
        change
          .lock()
          .unwrap()
          .push(format!("change {value:?} (the signal has {:?})", seen.get_untracked()))
      })
      .on_open_change(move |is| open.lock().unwrap().push(format!("open {is}")))
      .on_dismiss(move || dismiss.lock().unwrap().push("dismiss".to_owned()))
      .mount(ctx)
  });
  (stage, value, told)
}

#[test]
fn the_select_is_the_boards_trigger_and_its_panel_of_a_tree() {
  let mut stage = staged(|_| {
    TreeSelect::controlled("lib.rs", files(), |_| {})
      .inline(true)
      .id("select")
      .into()
  });
  // The trigger: a select's box, 260 by 36, its text 12 in at 14, its chevron of 16 12 from its end.
  assert_eq!(stage.bounds("select"), (8.0, 8.0, 260.0, 36.0));
  assert_eq!(stage.fill_color("select"), Some(light(PaletteColor::Background)));
  let edge = stage
    .border("select")
    .expect("the trigger's edge")
    .border
    .unwrap()
    .top
    .unwrap();
  assert_eq!((edge.width, edge.color), (1.0, light(PaletteColor::Input)));
  let text = stage
    .text_quad("crates / lurq / lib.rs")
    .expect("the path to what is chosen");
  assert_eq!(text.x, 20.0);
  assert_eq!(
    stage.text("crates / lurq / lib.rs"),
    (light(PaletteColor::Foreground), 14.0)
  );
  let chevron = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == CHEVRON_DOWN) && quad.y < 44.0)
    .expect("the trigger's chevron");
  assert_eq!((chevron.width, chevron.x + chevron.width), (16.0, 8.0 + 260.0 - 12.0));

  // The panel: 4 under the trigger, 260 wide on the popover's fill, 8 of padding around the tree.
  let (x, y, width, _) = stage.bounds("select-panel");
  assert_eq!((x, y, width), (8.0, 48.0, 260.0));
  assert_eq!(stage.fill_color("select-panel"), Some(light(PaletteColor::Popover)));
  let (x, y, width, height) = stage.bounds("crates");
  assert_eq!((x, y, width, height), (16.0, 56.0, 244.0, 28.0));
  // The branches the chosen row is in are open, and it is in the highlight.
  assert_eq!(stage.fill_color("lib.rs"), Some(light(PaletteColor::Highlight)));
  assert!(stage.paints("theme.rs") && !stage.paints("main.rs"));
}

#[test]
fn a_press_opens_the_panel_and_a_choice_closes_it_and_gives_the_focus_back() {
  let (mut stage, value, told) = bound(Some("lib.rs"));
  assert!(!stage.paints("theme.rs"));
  stage.click("select");
  assert_eq!(said(&told), ["open true"]);
  assert!(stage.paints("theme.rs"));
  // The focus is in the tree, on the chosen row.
  assert_eq!(focused(&stage).as_deref(), Some("lib.rs"));
  stage.click("theme.rs");
  assert_eq!(value.get(), Some("theme.rs"));
  // The panel closed first, then the handler heard the value, which the signal had already.
  assert_eq!(
    said(&told),
    [
      "open false",
      r#"change Some("theme.rs") (the signal has Some("theme.rs"))"#
    ]
  );
  assert!(!stage.paints("lib.rs"));
  assert!(stage.paints("crates / lurq / theme.rs"));
  assert_eq!(focused(&stage).as_deref(), Some("select"));
  // What the app sets is shown, and told to nobody.
  value.set(Some("Cargo.toml"));
  stage.pass();
  assert!(stage.paints("Cargo.toml"));
  assert!(said(&told).is_empty());
  // A press of the trigger closes an open panel.
  stage.click("select");
  stage.click("select");
  assert_eq!(said(&told), ["open true", "open false"]);
}

#[test]
fn the_keys_open_the_panel_move_through_the_tree_and_choose() {
  let (mut stage, value, told) = bound(Some("lib.rs"));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("select"));
  stage.key("ArrowDown");
  assert_eq!(said(&told), ["open true"]);
  assert_eq!(focused(&stage).as_deref(), Some("lib.rs"));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("theme.rs"));
  // Moving the focus chooses nothing.
  assert_eq!(value.get(), Some("lib.rs"));
  stage.key("Enter");
  assert_eq!(value.get(), Some("theme.rs"));
  assert_eq!(focused(&stage).as_deref(), Some("select"));
  assert_eq!(
    said(&told),
    [
      "open false",
      r#"change Some("theme.rs") (the signal has Some("theme.rs"))"#
    ]
  );
  // Enter on the trigger opens it too; Left closes a branch, and Enter on a branch opens it, choosing nothing.
  stage.key("Enter");
  assert_eq!(focused(&stage).as_deref(), Some("theme.rs"));
  stage.key("ArrowLeft");
  assert_eq!(focused(&stage).as_deref(), Some("lurq"));
  stage.key("ArrowLeft");
  assert!(!stage.paints("lib.rs"));
  stage.key("Enter");
  assert!(stage.paints("lib.rs"));
  assert_eq!(value.get(), Some("theme.rs"));
  said(&told);
  // Escape closes it and gives the focus back; the app hears a dismissal and then that it closed.
  stage.key("Escape");
  assert!(!stage.paints("lib.rs"));
  assert_eq!(focused(&stage).as_deref(), Some("select"));
  assert_eq!(said(&told), ["dismiss", "open false"]);
}

#[test]
fn a_press_outside_closes_the_panel() {
  let (mut stage, _, told) = bound(None);
  assert!(stage.paints("Select a file"));
  stage.click("select");
  said(&told);
  stage.click("after");
  assert!(!stage.paints("crates"));
  assert_eq!(said(&told), ["dismiss", "open false"]);
}

#[test]
fn a_select_of_one_or_none_shows_its_placeholder_and_is_cleared() {
  let value = Signal::new(Some("lib.rs"));
  let told: Told = Arc::default();
  let (held, heard) = (value.clone(), told.clone());
  let mut stage = staged(move |ctx| {
    let heard = heard.clone();
    TreeSelect::optional(held.clone(), files())
      .placeholder("Select a file")
      .clearable(true)
      .clear_label("Forget")
      .id("select")
      .on_change(move |value| heard.lock().unwrap().push(format!("{value:?}")))
      .mount(ctx)
  });
  // The cross stands before the chevron, and a press of it takes the value away without opening the panel.
  stage.click("select-clear");
  assert_eq!(value.get(), None);
  assert_eq!(said(&told), ["None"]);
  assert!(!stage.paints("crates"));
  assert_eq!(
    stage.text("Select a file"),
    (light(PaletteColor::MutedForeground), 14.0)
  );
  // With nothing chosen there is nothing to clear.
  assert!(stage.tree.get_element_by_id_mut("select-clear").is_none());
}

#[test]
fn a_select_of_any_number_adds_and_takes_out_with_a_press_and_stays_open() {
  let values = Signal::new(vec!["lib.rs"]);
  let told: Told = Arc::default();
  let (held, heard) = (values.clone(), told.clone());
  let mut stage = staged(move |ctx| {
    let heard = heard.clone();
    TreeSelect::multiple(held.clone(), files())
      .max_shown(2)
      .id("select")
      .on_change(move |values| heard.lock().unwrap().push(format!("{values:?}")))
      .mount(ctx)
  });
  assert!(stage.paints("lib.rs"));
  stage.click("select");
  stage.click("theme.rs");
  assert_eq!(values.get(), ["lib.rs", "theme.rs"]);
  assert_eq!(said(&told), [r#"["lib.rs", "theme.rs"]"#]);
  // The panel stays open, both rows in the highlight, and the trigger says both.
  assert_eq!(stage.fill_color("lib.rs"), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.fill_color("theme.rs"), Some(light(PaletteColor::Highlight)));
  assert!(stage.paints("lib.rs, theme.rs"));
  stage.click("Cargo.toml");
  assert!(stage.paints("lib.rs, theme.rs +1"));
  // A second press of a row takes it out, at once.
  stage.click("lib.rs");
  assert_eq!(values.get(), ["theme.rs", "Cargo.toml"]);
  // Space on the focused row does as much.
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("theme.rs"));
  stage.key(" ");
  assert_eq!(values.get(), ["Cargo.toml"]);
  // The rows taken out stay where they are: the branches opened with the panel do not close under the focus.
  assert_eq!(focused(&stage).as_deref(), Some("theme.rs"));
  assert!(stage.paints("lib.rs"));
  stage.key("Escape");
  assert_eq!(focused(&stage).as_deref(), Some("select"));
}

#[test]
fn where_only_leaves_are_chosen_a_press_of_a_branch_opens_or_closes_it() {
  let value = Signal::new(None);
  let held = value.clone();
  let mut stage = staged(move |ctx| {
    TreeSelect::optional(held.clone(), files())
      .leaves_only(true)
      .id("select")
      .mount(ctx)
  });
  stage.click("select");
  assert!(!stage.paints("lurq"));
  stage.click("crates");
  // It opened, chose nothing and the panel is still there.
  assert!(stage.paints("lurq"));
  assert_eq!(value.get(), None);
  stage.click("crates");
  assert!(!stage.paints("lurq"));
  stage.click("Cargo.toml");
  assert_eq!(value.get(), Some("Cargo.toml"));
}

#[test]
fn the_app_holds_the_open_branches_and_is_told_of_each() {
  let branches = Signal::new(vec!["crates"]);
  let told: Told = Arc::default();
  let (held, heard) = (branches.clone(), told.clone());
  let mut stage = staged(move |ctx| {
    let heard = heard.clone();
    TreeSelect::controlled("Cargo.toml", files(), |_| {})
      .branches(held.clone())
      .on_branch_change(move |id, open| heard.lock().unwrap().push(format!("{id} {open}")))
      .id("select")
      .mount(ctx)
  });
  stage.click("select");
  assert!(stage.paints("lurq") && !stage.paints("lib.rs"));
  assert_eq!(focused(&stage).as_deref(), Some("Cargo.toml"));
  stage.key("ArrowUp");
  stage.key("ArrowUp");
  assert_eq!(focused(&stage).as_deref(), Some("lurq"));
  stage.key("ArrowRight");
  assert_eq!(branches.get(), ["crates", "lurq"]);
  assert_eq!(said(&told), ["lurq true"]);
  assert!(stage.paints("lib.rs"));
}

#[test]
fn a_disabled_and_a_read_only_select_do_not_open() {
  for read_only in [false, true] {
    let mut stage = staged(move |ctx| {
      TreeSelect::controlled("lib.rs", files(), |_| {})
        .disabled(!read_only)
        .read_only(read_only)
        .id("select")
        .mount(ctx)
    });
    stage.key("Tab");
    stage.key("ArrowDown");
    assert!(!stage.paints("theme.rs"), "read only: {read_only}");
    // A read only one keeps the focus; a disabled one is passed by.
    assert_eq!(
      focused(&stage).as_deref(),
      Some(if read_only { "select" } else { "after" })
    );
    stage.click("select");
    assert!(!stage.paints("theme.rs"), "read only: {read_only}");
    assert_eq!(stage.fill_color("select"), Some(light(PaletteColor::Muted)));
  }
}

#[test]
fn the_sizes_and_the_judgments_are_a_selects() {
  for (size, height, type_size) in [
    (TreeSelectSize::Sm, 32.0, 13.0),
    (TreeSelectSize::Md, 36.0, 14.0),
    (TreeSelectSize::Lg, 40.0, 15.0),
  ] {
    let mut stage = staged(move |_| {
      TreeSelect::controlled("Cargo.toml", files(), |_| {})
        .size(size)
        .invalid(true)
        .id("select")
        .into()
    });
    assert_eq!(stage.bounds("select").3, height);
    assert_eq!(stage.text("Cargo.toml").1, type_size);
    let edge = stage.border("select").unwrap().border.unwrap().top.unwrap();
    assert_eq!(edge.color, light(PaletteColor::Destructive));
  }
  let mut stage = staged(|_| {
    TreeSelect::controlled("Cargo.toml", files(), |_| {})
      .force(Some(Interaction::FocusVisible))
      .id("select")
      .into()
  });
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  let _ = stage.bounds("select");
}

#[test]
fn the_trigger_says_the_label_alone_or_another_separator() {
  let stage = staged(|_| {
    Row::new()
      .child(TreeSelect::controlled("lib.rs", files(), |_| {}).path(false))
      .into()
  });
  assert!(stage.paints("lib.rs") && !stage.paints("crates / lurq / lib.rs"));
  let stage = staged(|_| {
    TreeSelect::controlled("lib.rs", files(), |_| {})
      .separator_text(" › ")
      .into()
  });
  assert!(stage.paints("crates › lurq › lib.rs"));
}

#[test]
fn a_path_longer_than_the_trigger_ends_in_an_ellipsis_and_the_chevron_keeps_its_place() {
  let long = "a-file-with-a-name-far-longer-than-the-trigger-of-a-select-is-wide.rs";
  let mut stage = staged(move |_| {
    TreeSelect::controlled("long", [folder("crates").item(TreeItem::file("long", long))], |_| {})
      .id("select")
      .into()
  });
  let said = format!("crates / {long}");
  let text = stage.cut_text_quad(&said).expect("the path, cut");
  let (x, _, width, _) = stage.bounds("select");
  assert_eq!(width, 260.0);
  assert!(
    text.x + text.width <= x + width - 12.0 - 16.0 - 8.0 + 0.5,
    "{}",
    text.width
  );
  let chevron = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == CHEVRON_DOWN))
    .unwrap();
  assert_eq!(chevron.x + chevron.width, x + width - 12.0);
}

#[test]
fn a_full_width_select_fills_its_room_and_its_panel_is_as_wide() {
  let mut stage = staged(|ctx| {
    TreeSelect::controlled("lib.rs", files(), |_| {})
      .full_width(true)
      .id("select")
      .mount(ctx)
  });
  // The room beside the button: more than the design's 260.
  let width = stage.bounds("select").2;
  assert!(width > 300.0, "{width}");
  stage.click("select");
  assert_eq!(stage.bounds("select-panel").2, width);
}

#[test]
fn a_select_with_no_items_says_so() {
  let mut stage = staged(|ctx| {
    TreeSelect::controlled_optional(None::<Id>, [], |_| {})
      .empty_text("Nothing to choose")
      .id("select")
      .mount(ctx)
  });
  stage.click("select");
  assert_eq!(
    stage.text("Nothing to choose"),
    (light(PaletteColor::MutedForeground), 13.0)
  );
}

#[test]
fn a_field_names_and_judges_the_select() {
  let mut stage = staged(|ctx| {
    Field::new(
      "Entry point",
      TreeSelect::controlled("lib.rs", files(), |_| {}).id("select"),
    )
    .error("Choose a file of the crate")
    .mount(ctx)
  });
  let edge = stage.border("select").unwrap().border.unwrap().top.unwrap();
  assert_eq!(edge.color, light(PaletteColor::Destructive));
  // Mounted by the field, it opens.
  stage.click("select");
  assert!(stage.paints("theme.rs"));
}

#[test]
fn a_theme_gives_the_select_another_anatomy_and_another_rule() {
  let app = App::new();
  let renderer: TreeSelectRenderer = Arc::new(|view: TreeSelectView| {
    Row::new()
      .child(Text::new(&format!(
        "chosen: {}",
        view.text.as_deref().unwrap_or("nothing")
      )))
      .into()
  });
  app.theme().set_renderer::<TreeSelect<Id>>(renderer);
  let stage = Stage::in_app(app, |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(TreeSelect::controlled("lib.rs", files(), |_| {}))
      .into()
  });
  assert!(stage.paints("chosen: crates / lurq / lib.rs"));

  let app = App::new();
  app
    .theme()
    .patch_recipe::<TreeSelect<Id>>(|recipe| recipe.base(TreeSelectPart::Trigger, |s| s.width(300.0)));
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(TreeSelect::controlled("lib.rs", files(), |_| {}).id("select"))
      .into()
  });
  assert_eq!(stage.bounds("select").2, 300.0);
  assert_eq!(<TreeSelect as Themed>::NAME, "tree_select");
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_combobox_that_opens_a_tree() {
  let read = |stage: &mut Stage, id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  let (mut stage, ..) = bound(Some("lib.rs"));
  assert_eq!(read(&mut stage, "select", "a11y_role").as_deref(), Some("combobox"));
  assert_eq!(read(&mut stage, "select", "a11y_name").as_deref(), Some("File"));
  assert_eq!(read(&mut stage, "select", "a11y_haspopup").as_deref(), Some("tree"));
  assert_eq!(read(&mut stage, "select", "a11y_expanded").as_deref(), Some("false"));
  assert_eq!(
    read(&mut stage, "select", "a11y_value").as_deref(),
    Some("crates / lurq / lib.rs")
  );
  stage.click("select");
  assert_eq!(read(&mut stage, "select", "a11y_expanded").as_deref(), Some("true"));
  assert_eq!(read(&mut stage, "select-panel", "a11y_role").as_deref(), Some("dialog"));
  assert_eq!(read(&mut stage, "select-tree", "a11y_role").as_deref(), Some("tree"));
  assert_eq!(read(&mut stage, "select-tree", "a11y_name").as_deref(), Some("File"));
  assert_eq!(read(&mut stage, "lib.rs", "a11y_selected").as_deref(), Some("true"));
}
