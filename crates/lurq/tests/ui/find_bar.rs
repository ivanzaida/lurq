//! A find bar: its panel, field, options, count and buttons against the
//! board, where it hangs, the keys of the field and of the bar, an option
//! turned on, the replace row, the dismissal and where the focus goes, and
//! what it is to tooling.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    theme::{MotionDuration, MotionEasing, PaletteColor, ThemePalette},
  },
  components::{Column, Stack},
  core::{ElementRef, Signal},
  node::color::Color,
  ui::{FADE_IN, FindBar, FindBarPart, PartAnimation},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

/// What the bar was told, in order: "next", "previous", "query:…", "replace:…", "dismiss".
type Log = Arc<Mutex<Vec<String>>>;

fn note(log: &Log, what: impl Into<String>) {
  log.lock().unwrap().push(what.into());
}

/// An app whose find bar fades in where it hangs: the toast's keyframes it moves in with run on the process's
/// clock, so where it is while it moves depends on when a test reads it.
fn still() -> App {
  let app = App::new();
  app.theme().patch_recipe::<FindBar>(|recipe| {
    recipe.base(FindBarPart::Bar, |s| {
      s.enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
    })
  });
  app
}

/// The bar over a view of 400 by 300 with a button in it: `adjust` says what else it has.
fn stage(log: &Log, adjust: impl Fn(FindBar) -> FindBar + Send + Sync + 'static) -> (Stage, Signal<String>) {
  let query = Signal::new(String::new());
  let (held, log) = (query.clone(), log.clone());
  let stage = Stage::in_app(still(), move |ctx| {
    let (next, previous, typed, dismissed) = (log.clone(), log.clone(), log.clone(), log.clone());
    let bar = FindBar::new(held.clone())
      .id("find")
      .on_next(move || note(&next, "next"))
      .on_previous(move || note(&previous, "previous"))
      .on_query(move |text| note(&typed, format!("query:{text}")))
      .on_dismiss(move || note(&dismissed, "dismiss"));
    Stack::new()
      .size(400.0, 300.0)
      .id("view")
      .child(
        Column::new()
          .size(400.0, 300.0)
          .child(lurq::components::Button::new("Text").id("text")),
      )
      .child(adjust(bar).mount(ctx))
      .into()
  });
  (stage, query)
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
fn describe(stage: &mut Stage, id: &str, key: &str) -> Option<String> {
  stage
    .tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.description(key))
}

fn shift_key(stage: &mut Stage, key: &str) {
  stage.tree.key_down(key.to_owned(), key.to_owned(), true, false, false);
  stage.tree.key_up(key.to_owned(), key.to_owned(), true, false, false);
  stage.pass();
}

#[test]
fn it_hangs_from_the_top_end_of_its_view_as_the_board_draws_it() {
  let log = Log::default();
  let (mut stage, _) = stage(&log, |bar| {
    bar
      .matches(3, 12)
      .match_case(Signal::new(true))
      .regex(Signal::new(false))
  });
  let (x, y, width, _) = stage.bounds("find");
  assert_eq!(width, 384.0, "the board's 440, in a view of 400 less 8 on either side");
  assert!(close(y, 8.0), "8 from the top");
  assert!(close(x + width, 392.0), "8 from the end");
  assert_eq!(stage.fill_color("find"), Some(light(PaletteColor::Popover)));
  // The field is 28 high, padded 6 in the panel; the buttons are 24 square.
  let (_, field_y, _, field_height) = stage.bounds("find-field");
  assert_eq!(field_height, 28.0);
  assert!(close(field_y - y, 6.0), "padded 6");
  for button in ["find-previous", "find-next", "find-close"] {
    let (_, _, width, height) = stage.bounds(button);
    assert_eq!((width, height), (24.0, 24.0), "{button}");
  }
  // The options are 22 square over the field's end; the one that is on has the fill of what is picked out.
  let (case_x, _, case_width, case_height) = stage.bounds("find-match-case");
  let (field_x, _, field_width, _) = stage.bounds("find-field");
  assert_eq!((case_width, case_height), (22.0, 22.0));
  assert!(
    case_x > field_x && case_x + 22.0 < field_x + field_width,
    "inside the field"
  );
  assert_eq!(
    stage.fill_color("find-match-case"),
    Some(light(PaletteColor::Highlight))
  );
  assert_eq!(stage.fill_color("find-regex"), None);
  assert!(
    stage.tree.get_element_by_id_mut("find-whole-word").is_none() && !stage.paints("ab"),
    "only the options given"
  );
  // The count is 11 in the muted foreground.
  assert_eq!(stage.text("3 of 12"), (light(PaletteColor::MutedForeground), 11.0));
}

#[test]
fn it_opens_with_the_focus_in_its_field_where_enter_goes_on_and_shift_enter_back() {
  let log = Log::default();
  let (mut stage, query) = stage(&log, |bar| bar);
  assert_eq!(focused(&stage).as_deref(), Some("find-field"));
  stage.tree.text_input_replace(0..0, "lurq");
  stage.pass();
  assert_eq!(query.get(), "lurq");
  stage.key("Enter");
  shift_key(&mut stage, "Enter");
  stage.key("F3");
  shift_key(&mut stage, "F3");
  let told: Vec<String> = log.lock().unwrap().clone();
  assert_eq!(told, ["query:lurq", "next", "previous", "next", "previous"]);
}

#[test]
fn the_options_show_what_the_app_says() {
  let log = Log::default();
  let (stage, _) = stage(&log, |bar| {
    bar
      .match_case(Signal::new(false))
      .whole_word(Signal::new(false))
      .regex(Signal::new(false))
      .match_case_text("Cc")
      .whole_word_text("W")
      .regex_text("Re")
  });
  for text in ["Cc", "W", "Re"] {
    assert!(stage.paints(text), "{text}");
  }
  assert!(!stage.paints("Aa") && !stage.paints(".*"));
}

#[test]
fn the_count_says_what_the_app_gives_it() {
  let log = Log::default();
  let (mut none, query) = stage(&log, |bar| bar.matches(0, 0));
  query.set("zzz".to_owned());
  none.pass();
  assert_eq!(none.text("No results").0, light(PaletteColor::Destructive));
  drop(none);
  let (worded, _) = stage(&log, |bar| {
    bar
      .matches(2, 5)
      .count_text(|current, total| format!("{current}/{total}"))
  });
  assert!(worded.paints("2/5"));
}

#[test]
fn an_option_pressed_sets_its_signal_and_says_it_is_pressed() {
  let log = Log::default();
  let case = Signal::new(false);
  let told = Arc::new(AtomicUsize::new(0));
  let (held, count) = (case.clone(), told.clone());
  let (mut stage, _) = stage(&log, move |bar| {
    let count = count.clone();
    bar.match_case(held.clone()).on_match_case_change(move |on| {
      assert!(on);
      count.fetch_add(1, Ordering::SeqCst);
    })
  });
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  assert_eq!(
    describe(&mut stage, "find-match-case", "a11y_pressed").as_deref(),
    Some("false")
  );
  stage.click("find-match-case");
  assert!(case.get());
  assert_eq!(told.load(Ordering::SeqCst), 1);
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  assert_eq!(
    describe(&mut stage, "find-match-case", "a11y_pressed").as_deref(),
    Some("true")
  );
  assert_eq!(
    stage.fill_color("find-match-case"),
    Some(light(PaletteColor::Highlight))
  );
}

#[test]
fn the_replace_row_shows_under_the_chevron_and_replaces_with_its_text() {
  let log = Log::default();
  let (open, text) = (Signal::new(false), Signal::new(String::new()));
  let (held_open, held_text) = (open.clone(), text.clone());
  let replaced = log.clone();
  let all = log.clone();
  let (mut stage, _) = stage(&log, move |bar| {
    let (replaced, all) = (replaced.clone(), all.clone());
    bar
      .replace(held_text.clone())
      .replace_open(held_open.clone())
      .on_replace(move |text| note(&replaced, format!("replace:{text}")))
      .on_replace_all(move |text| note(&all, format!("all:{text}")))
  });
  assert!(stage.tree.get_element_by_id_mut("find-replace-field").is_none());
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  assert_eq!(
    describe(&mut stage, "find-toggle-replace", "a11y_expanded").as_deref(),
    Some("false")
  );
  stage.click("find-toggle-replace");
  assert!(open.get());
  stage.pass();
  let (field_x, ..) = stage.bounds("find-field");
  let (replace_x, _, _, replace_height) = stage.bounds("find-replace-field");
  assert_eq!(replace_height, 28.0);
  assert!(close(replace_x, field_x), "indented under the field");
  stage.click("find-replace-field");
  stage.tree.text_input_replace(0..0, "core");
  stage.pass();
  stage.key("Enter");
  stage.click("find-replace-all");
  let told: Vec<String> = log.lock().unwrap().clone();
  assert_eq!(told, ["replace:core", "all:core"]);
}

#[test]
fn escape_and_the_close_button_dismiss_it_and_the_focus_goes_where_the_app_says() {
  let log = Log::default();
  let open = Signal::new(false);
  let target = ElementRef::new();
  let (held, back) = (open.clone(), target.clone());
  let query = Signal::new(String::new());
  let (q, l) = (query.clone(), log.clone());
  let mut stage = Stage::in_app(still(), move |ctx| {
    let dismissed = l.clone();
    Stack::new()
      .size(400.0, 300.0)
      .child(
        Column::new().size(400.0, 300.0).child(
          lurq::components::Button::new("Text")
            .id("text")
            .ref_element(back.clone()),
        ),
      )
      .child(
        FindBar::new(q.clone())
          .id("find")
          .open(held.clone())
          .return_focus(back.clone())
          .on_dismiss(move || note(&dismissed, "dismiss"))
          .mount(ctx),
      )
      .into()
  });
  // Open from the start the bar would leave the focus where it is: it takes it as it opens.
  assert_eq!(focused(&stage), None);
  open.set(true);
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("find-field"));
  stage.key("Escape");
  assert!(!open.get());
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("text"));
  // Opened again, the focus is in the field; the close button closes it as Escape does.
  open.set(true);
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("find-field"));
  stage.click("find-close");
  assert!(!open.get());
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("text"));
  assert_eq!(log.lock().unwrap().as_slice(), ["dismiss", "dismiss"]);
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn it_is_a_search_and_each_of_its_controls_is_called_by_its_label() {
  let log = Log::default();
  let (mut stage, _) = stage(&log, |bar| {
    bar
      .label("Find in page")
      .matches(1, 2)
      .match_case(Signal::new(false))
      .whole_word(Signal::new(false))
      .regex(Signal::new(false))
      .replace(Signal::new(String::new()))
      .next_label("Next")
  });
  assert_eq!(describe(&mut stage, "find", "a11y_role").as_deref(), Some("search"));
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  assert_eq!(
    describe(&mut stage, "find", "a11y_name").as_deref(),
    Some("Find in page")
  );
  for (id, name) in [
    ("find-previous", "Previous match"),
    ("find-next", "Next"),
    ("find-close", "Close"),
    ("find-match-case", "Match case"),
    ("find-whole-word", "Match whole word"),
    ("find-regex", "Use regular expression"),
    ("find-replace", "Replace"),
    ("find-replace-all", "Replace all"),
  ] {
    assert_eq!(describe(&mut stage, id, "a11y_name").as_deref(), Some(name), "{id}");
  }
}
