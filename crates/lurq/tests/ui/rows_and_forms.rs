//! What Orchester's page tabs, rows, fields, dialogs and buttons need of lurq's: a tab's note at the end, its dot, a
//! count still read and a tab a press leaves alone; a list item's tags, (i), details and what it says under itself; a
//! field with no label and rows under it; a dialog's key caps in its note and a veil that leaves a band at the top; a
//! button whose label takes its room, whose description wraps, and that tells its focus.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::{Column, Row, TextInput},
  core::Signal,
  node::Element,
  ui::{Button, Dialog, Field, Input, List, ListItem, Tab, Tabs, TabsVariant, Tag, icons},
};

use super::stage::Stage;

fn counter() -> (Arc<AtomicUsize>, impl Fn() + Send + Sync + Clone + 'static) {
  let count = Arc::new(AtomicUsize::new(0));
  let counted = count.clone();
  (count, move || {
    counted.fetch_add(1, Ordering::SeqCst);
  })
}

fn count(counter: &AtomicUsize) -> usize {
  counter.load(Ordering::SeqCst)
}

fn scene(content: impl Fn() -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(move |_| Column::new().size(400.0, 300.0).child(content()).into());
  stage.tree.resize(400, 300);
  stage.pass();
  stage
}

fn has(stage: &Stage, id: &str) -> bool {
  stage.ids().iter().any(|held| held == id)
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn tabs_have_a_note_and_an_element_of_the_app_at_the_end_of_their_line() {
  let mut stage = scene(|| {
    Tabs::new(
      Signal::new(0u8),
      [Tab::new(0, "All").id("all"), Tab::new(1, "Open").id("open")],
    )
    .variant(TabsVariant::Line)
    .note("Archived tabs open in their own page")
    .end(Row::new().size(20.0, 20.0).id("act"))
    .id("t")
    .into()
  });
  assert!(has(&stage, "t-note") && has(&stage, "t-end"), "{:?}", stage.ids());
  let (open_x, _, open_width, _) = stage.bounds("open");
  let note = stage
    .text_quad("Archived tabs open in their own page")
    .expect("the note");
  let (act_x, _, act_width, _) = stage.bounds("act");
  assert!(note.x >= open_x + open_width, "the note after the tabs");
  assert!(act_x >= note.x + note.width, "the element after the note");
  assert!(
    (act_x + act_width - 400.0).abs() < 1.0,
    "at the line's end: {}",
    act_x + act_width
  );
}

#[test]
fn a_tab_has_a_dot_in_the_color_it_is_given_after_its_count() {
  let mut stage = scene(|| {
    Tabs::new(
      Signal::new(0u8),
      [Tab::new(0, "Services").id("s").count(3).dot(PaletteColor::Destructive)],
    )
    .variant(TabsVariant::Line)
    .into()
  });
  assert_eq!(
    stage.fill_color("s-dot"),
    Some(ThemePalette::light().get(PaletteColor::Destructive))
  );
  let (dot_x, _, dot_width, dot_height) = stage.bounds("s-dot");
  let (count_x, ..) = stage.bounds("s-count");
  assert!(dot_x > count_x, "after the count");
  assert_eq!((dot_width, dot_height), (6.0, 6.0));
}

#[test]
fn a_tab_whose_count_is_read_has_a_bone_in_its_place_until_the_count_comes() {
  let reading = |count: Option<u32>| {
    scene(move || {
      let tab = Tab::new(0u8, "Runs").id("runs").count_pending(true);
      let tab = match count {
        Some(count) => tab.count(count),
        None => tab,
      };
      Tabs::new(Signal::new(0u8), [tab]).variant(TabsVariant::Line).into()
    })
  };
  let mut stage = reading(None);
  assert!(has(&stage, "runs-count") && has(&stage, "runs-count_skeleton"));
  let (_, _, bone_width, _) = stage.bounds("runs-count");
  let stage_counted = reading(Some(4));
  assert!(!has(&stage_counted, "runs-count_skeleton"), "the count in its place");
  assert!(stage_counted.paints("4"));
  assert!((6.0..=9.0).contains(&bone_width), "a digit's room: {bone_width}");
}

#[test]
fn a_tab_a_press_leaves_alone_is_neither_selected_nor_focused_and_the_arrows_pass_it_by() {
  let selected = Signal::new(0u8);
  let (changed, change) = counter();
  let held = selected.clone();
  let mut stage = scene(move || {
    let change = change.clone();
    Tabs::new(
      held.clone(),
      [
        Tab::new(0, "One").id("one"),
        Tab::new(1, "Two").id("two").inert(true),
        Tab::new(2, "Three").id("three"),
      ],
    )
    .on_change(move |_| change())
    .into()
  });
  stage.click("two");
  assert_eq!((selected.get(), count(&changed)), (0, 0), "a press selects nothing");
  assert_ne!(focused(&stage).as_deref(), Some("two"), "and focuses nothing");
  stage.click("one");
  stage.key("ArrowRight");
  assert_eq!(selected.get(), 2, "the arrow passes the inert tab by");
  assert_eq!(focused(&stage).as_deref(), Some("three"));
}

#[test]
fn a_selected_tab_a_press_leaves_alone_stays_drawn_as_selected() {
  let mut stage = scene(|| {
    Tabs::new(
      Signal::new(0u8),
      [
        Tab::new(0, "Here").id("here").inert(true),
        Tab::new(1, "There").id("there"),
      ],
    )
    .into()
  });
  // The default form lifts the selected tab out of its rail in the background's color.
  assert_eq!(
    stage.fill_color("here"),
    Some(ThemePalette::light().get(PaletteColor::Background))
  );
}

#[test]
fn a_row_has_tags_and_a_mark_after_its_title_and_a_detail_under_its_description() {
  let mut stage = scene(|| {
    ListItem::new("Theme")
      .description("How the app looks")
      .tag(Tag::new("Default"))
      .info("Follows the system unless set")
      .detail(Row::new().size(60.0, 14.0).id("link"))
      .id("r")
      .into()
  });
  for id in ["r-title", "r-description", "r-tags", "r-info", "r-detail"] {
    assert!(has(&stage, id), "{id} in {:?}", stage.ids());
  }
  let (title_x, _, title_width, _) = stage.bounds("r-title");
  let (tags_x, ..) = stage.bounds("r-tags");
  let (info_x, ..) = stage.bounds("r-info");
  assert!(tags_x >= title_x + title_width && info_x > tags_x, "title, tags, mark");
  let (_, description_y, _, description_height) = stage.bounds("r-description");
  let (_, link_y, ..) = stage.bounds("link");
  assert!(
    link_y >= description_y + description_height,
    "the detail under the description"
  );
}

#[test]
fn a_row_s_subtitle_stands_between_its_title_and_its_description_and_a_detail_after_them() {
  let (rows, press_row) = counter();
  let (paths, press_path) = counter();
  let mut stage = scene(move || {
    let (press_row, press_path) = (press_row.clone(), press_path.clone());
    ListItem::new("Program path")
      .subtitle(
        Button::new("/Applications/Chrome.app")
          .id("path")
          .on_click(move |_| press_path()),
      )
      .description("Leave empty to find it on its own")
      .detail(Row::new().size(60.0, 14.0).id("link"))
      .on_click(move |_| press_row())
      .id("r")
      .into()
  });
  for id in ["r-title", "r-subtitle", "r-description", "r-detail"] {
    assert!(has(&stage, id), "{id} in {:?}", stage.ids());
  }
  let (_, title_y, _, title_height) = stage.bounds("r-title");
  let (_, path_y, _, path_height) = stage.bounds("path");
  let (_, description_y, _, description_height) = stage.bounds("r-description");
  let (_, link_y, ..) = stage.bounds("link");
  assert!(path_y >= title_y + title_height, "the subtitle under the title");
  assert!(description_y >= path_y + path_height, "the description under the subtitle");
  assert!(link_y >= description_y + description_height, "the detail under the description");
  stage.click("path");
  assert_eq!((count(&paths), count(&rows)), (1, 0), "a press of the subtitle is its own");
}

#[test]
fn what_a_row_says_under_itself_stands_under_its_line_and_is_not_a_press_of_the_row() {
  let (rows, press_row) = counter();
  let (fixes, fix) = counter();
  let mut stage = scene(move || {
    let (press_row, fix) = (press_row.clone(), fix.clone());
    List::new()
      .contained(true)
      .item(
        ListItem::new("Token")
          .description("Expired")
          .on_click(move |_| press_row())
          .below(Button::new("Renew").id("renew").on_click(move |_| fix()))
          .id("r"),
      )
      .item(ListItem::new("Next").id("next"))
      .into()
  });
  assert!(has(&stage, "r-below"));
  let (_, row_y, _, row_height) = stage.bounds("r");
  let (_, below_y, ..) = stage.bounds("r-below");
  let (_, next_y, ..) = stage.bounds("next");
  assert!(below_y >= row_y + row_height - 0.5, "under the row's line");
  assert!(next_y > below_y, "the next row after it");
  stage.click("renew");
  assert_eq!((count(&fixes), count(&rows)), (1, 0));
  stage.click("r");
  assert_eq!(count(&rows), 1);
}

#[test]
fn a_field_with_no_label_draws_none_and_takes_rows_under_its_description() {
  let mut stage = scene(|| {
    Field::unlabeled(Input::new(Signal::new(String::new())).label("Search").full_width(true))
      .description("Finds by title")
      .below(Row::new().size(80.0, 12.0).id("err"))
      .below(Row::new().size(80.0, 12.0).id("err2"))
      .id("f")
      .into()
  });
  assert!(!has(&stage, "f-label"), "{:?}", stage.ids());
  assert!(has(&stage, "f-below"));
  let (_, description_y, _, description_height) = stage.bounds("f-description");
  let (_, err_y, _, err_height) = stage.bounds("err");
  let (_, err2_y, ..) = stage.bounds("err2");
  assert!(err_y >= description_y + description_height, "under the description");
  assert!(err2_y >= err_y + err_height, "one under another");
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn a_control_in_a_field_with_no_label_keeps_its_own_name() {
  let mut stage = scene(|| {
    Field::unlabeled(lurq::ui::Switch::new("Dark mode").label_hidden(true).id("dark"))
      .id("f")
      .into()
  });
  let name = stage
    .tree
    .get_element_by_id_mut("dark")
    .and_then(|element| element.description("a11y_name"));
  assert_eq!(name.as_deref(), Some("Dark mode"));
}

#[test]
fn a_dialog_note_holds_key_caps_with_their_words_in_order() {
  let mut stage = scene(|| {
    Dialog::new(Signal::new(true))
      .title("Delete run?")
      .note_key("Enter", "Delete")
      .note_key("Esc", "cancels")
      .footer(Button::new("Delete").id("delete"))
      .inline(true)
      .id("d")
      .into()
  });
  assert!(has(&stage, "d-note") && has(&stage, "d-note_key") && has(&stage, "d-note_key-2"));
  let (first_x, ..) = stage.bounds("d-note_key");
  let (second_x, ..) = stage.bounds("d-note_key-2");
  let (delete_x, ..) = stage.bounds("delete");
  let words = stage.text_quad("cancels").expect("the words of Esc");
  assert!(first_x < second_x && words.x > second_x, "in order");
  assert!(words.x + words.width <= delete_x, "before the buttons");
}

#[test]
fn a_dialog_veil_leaves_the_band_at_the_top_to_what_is_under_it() {
  let (bars, press_bar) = counter();
  let open = Signal::new(true);
  let shown = open.clone();
  let mut stage = scene(move || {
    let press_bar = press_bar.clone();
    Column::new()
      .size(400.0, 300.0)
      .child(
        Button::new("Bar")
          .id("bar")
          .style("root", |s| s.height(30.0))
          .on_click(move |_| press_bar()),
      )
      .child(Dialog::new(shown.clone()).title("Rename").inset_top(40.0).id("d"))
      .into()
  });
  let (_, panel_y, _, panel_height) = stage.bounds("d");
  let middle = panel_y + panel_height / 2.0;
  assert!((middle - 170.0).abs() < 1.5, "in the middle of the rest: {middle}");
  stage.click("bar");
  assert_eq!(count(&bars), 1, "the band takes the pointer");
  assert!(open.get(), "and a press there is no press of the veil");
}

#[test]
fn a_button_whose_label_fills_puts_what_is_after_it_at_its_end() {
  let mut stage = scene(|| {
    Column::new()
      .width(300.0)
      .child(
        Button::new("Pick")
          .variant("outline")
          .trailing(Row::new().size(14.0, 14.0).id("chevron"))
          .fill_label(true)
          .full_width(true)
          .id("b"),
      )
      .into()
  });
  let (button_x, _, button_width, _) = stage.bounds("b");
  let label = stage.text_quad("Pick").expect("the label");
  let (chevron_x, _, chevron_width, _) = stage.bounds("chevron");
  assert!(
    label.x - button_x < 20.0,
    "the label at the start: {}",
    label.x - button_x
  );
  let end = button_x + button_width - (chevron_x + chevron_width);
  assert!(end < 20.0, "the chevron at the end: {end} from it");
}

#[test]
fn a_button_whose_description_wraps_takes_lines_in_its_room() {
  let words = "Moves the task back to the queue and tells the person who asked for it";
  let built = |wraps: bool| {
    scene(move || {
      Column::new()
        .width(220.0)
        .child(
          Button::new("Send back")
            .variant("outline")
            .description(words)
            .description_wraps(wraps)
            .full_width(true)
            .id("b"),
        )
        .into()
    })
  };
  let (mut wrapped, mut cut) = (built(true), built(false));
  let (_, _, wrapped_width, wrapped_height) = wrapped.bounds("b-description");
  let (_, _, _, cut_height) = cut.bounds("b-description");
  assert!(
    wrapped_height > cut_height * 1.8,
    "{wrapped_height} against {cut_height}"
  );
  assert!(wrapped_width <= 220.0, "in the button's room");
}

#[test]
fn a_button_tells_the_app_it_took_the_focus() {
  let (focuses, focus) = counter();
  let mut stage = scene(move || {
    let focus = focus.clone();
    Column::new()
      .child(
        TextInput::new(Signal::new(String::new()))
          .id("field")
          .width(160.0)
          .tab_index(0),
      )
      .child(
        Button::new("Answer")
          .id("answer")
          .icon(icons::CHECK)
          .on_focus(focus),
      )
      .into()
  });
  stage.click("field");
  assert_eq!(count(&focuses), 0);
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("answer"));
  assert_eq!(count(&focuses), 1);
}
