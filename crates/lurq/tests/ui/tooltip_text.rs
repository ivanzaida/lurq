//! What a tooltip says and how: the width it wraps at, a note under its text,
//! a text in mono, a tooltip that shows only while a label is cut, and the
//! label of an icon-only button as its tooltip outside a toolbar.

use std::time::{Duration, Instant};

use lurq::{
  components::{Column, Placement, Row, Text, TextOverflow},
  core::{ElementRef, Signal},
  layout::quad::{Quad, QuadContent},
  ui::{Button, ButtonVariant, Tooltip, icons},
};

use super::stage::Stage;

/// A tooltip with no delay on an outlined button, 100 in from the window's corner.
fn stage(tooltip: impl Fn() -> Tooltip + Send + Sync + 'static) -> Stage {
  Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(100.0)
      .child(tooltip().mount(ctx, Button::new("Copy").variant(ButtonVariant::Outline).id("copy")))
      .into()
  })
}

/// The quad of the first line painted of `text`, wrapped or not.
fn first_line(stage: &Stage, text: &str) -> Quad {
  stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text: line, .. } if !line.is_empty() && text.starts_with(line.as_str())))
    .unwrap_or_else(|| panic!("no line of {text:?} was painted"))
}

/// The lines painted of `text`.
fn lines(stage: &Stage, text: &str) -> Vec<Quad> {
  let words: Vec<&str> = text.split_whitespace().collect();
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Text { text: line, .. }
        if !line.trim().is_empty() && line.split_whitespace().all(|word| words.contains(&word)))
    })
    .collect()
}

/// Lets the timers of the tooltips run `later` from now, and paints.
fn after(stage: &mut Stage, later: Duration) {
  stage.tree.tick_timers_at(Instant::now() + later);
  stage.pass();
}

const PATH: &str = "/Users/ada/projects/orchester/.orchester/worktrees/run-42/crates/desktop-app/src/ui/tooltip.rs";

#[test]
fn a_max_width_is_where_its_text_wraps_in_place_of_the_recipes() {
  let mut narrow = stage(|| Tooltip::new(PATH).open_delay(Duration::ZERO));
  let mut wide = stage(|| Tooltip::new(PATH).open_delay(Duration::ZERO).max_width(600.0));
  narrow.hover("copy");
  wide.hover("copy");
  let (narrow, wide) = (first_line(&narrow, PATH), first_line(&wide, PATH));
  assert!(
    narrow.width <= 200.0,
    "the recipe's 200 less its padding: {}",
    narrow.width
  );
  assert!(
    wide.width > 250.0,
    "past the recipe's 200, as far as the window lets it: {}",
    wide.width
  );
}

#[test]
fn a_note_is_a_second_line_under_the_text_which_is_its_title() {
  let note = "Signed in as ada@example.com on this computer";
  let mut stage = stage(move || Tooltip::new("Switch account").note(note).open_delay(Duration::ZERO));
  stage.hover("copy");
  let title = stage.text_quad("Switch account").expect("its title shows");
  let under = lines(&stage, note);
  assert!(!under.is_empty(), "its note shows");
  let first = &under[0];
  assert!(
    first.y >= title.y + title.height + 1.5,
    "2 under the title: {:?} under {:?}",
    (first.x, first.y),
    (title.x, title.y, title.height)
  );
  assert!((first.x - title.x).abs() < 0.5, "at the title's start");
  assert_eq!(stage.text("Switch account").1, 12.0);
  for line in &under {
    assert!(line.width <= 200.0, "the note wraps in the box: {}", line.width);
  }
}

#[test]
fn a_mono_tooltip_sets_its_text_in_the_mono_family() {
  let mut plain = stage(|| Tooltip::new("cargo test").open_delay(Duration::ZERO));
  let mut mono = stage(|| Tooltip::new("cargo test").mono(true).open_delay(Duration::ZERO));
  plain.hover("copy");
  mono.hover("copy");
  assert!(!plain.font_of("cargo test").to_lowercase().contains("mono"));
  assert!(mono.font_of("cargo test").to_lowercase().contains("mono"));
}

/// A label 12 of type in a row `width` wide, cut with an ellipsis where it does not fit, and its tooltip that
/// shows only while it is cut.
fn label_stage(width: f32, label: &'static str) -> (Stage, ElementRef) {
  let text = ElementRef::new();
  let held = text.clone();
  let stage = Stage::new(move |ctx| {
    let words = Text::new(label)
      .id("label")
      .nowrap()
      .text_overflow(TextOverflow::Elipsis)
      .ref_element(held.clone());
    Column::new()
      .size(400.0, 300.0)
      .padding(40.0)
      .child(
        Row::new().width(width).child(
          Tooltip::new(label)
            .when_cut(&held)
            .open_delay(Duration::ZERO)
            .max_width(600.0)
            .id("whole")
            .mount(ctx, words),
        ),
      )
      .into()
  });
  (stage, text)
}

const LONG: &str = "The pipeline of the release of the desktop app";

#[test]
fn a_tooltip_when_cut_shows_while_its_label_is_cut() {
  let (mut stage, text) = label_stage(120.0, LONG);
  assert!(text.text_cut(), "the label is cut to its room");
  assert!(!stage.paints(LONG), "nothing shows at rest");
  stage.hover("label");
  assert!(stage.paints(LONG), "the label whole in its tooltip");
}

#[test]
fn a_tooltip_when_cut_shows_nothing_while_its_label_fits() {
  let (mut stage, text) = label_stage(300.0, "Release");
  assert!(!text.text_cut(), "the label fits");
  stage.hover("label");
  stage.pass();
  assert!(stage.tree.get_element_by_id_mut("whole").is_none(), "no tooltip shows");
}

#[test]
fn a_texts_ref_says_whether_the_layout_cut_it() {
  let (cut, whole) = (ElementRef::new(), ElementRef::new());
  let (held_cut, held_whole) = (cut.clone(), whole.clone());
  let _stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 200.0)
      .child(
        Row::new().width(60.0).child(
          Text::new(LONG)
            .nowrap()
            .text_overflow(TextOverflow::Elipsis)
            .ref_element(held_cut.clone()),
        ),
      )
      .child(Text::new("Run").ref_element(held_whole.clone()))
      .into()
  });
  assert!(cut.text_cut());
  assert!(!whole.text_cut());
  assert!(!ElementRef::new().text_cut(), "nothing laid out is not cut");
}

#[test]
fn an_icon_only_button_mounted_has_its_label_as_its_tooltip() {
  let mut stage = Stage::new(|ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(100.0)
      .spacing(8.0)
      .child(
        Button::new("Settings")
          .icon(icons::SETTINGS)
          .icon_only(true)
          .id("cog")
          .mount(ctx),
      )
      .child(
        Button::new("Search")
          .icon(icons::SEARCH)
          .icon_only(true)
          .tooltip(
            Tooltip::new("Search the page")
              .shortcut("Ctrl+F")
              .open_delay(Duration::ZERO),
          )
          .id("find")
          .mount(ctx),
      )
      .child(Button::new("Save").id("save").mount(ctx))
      .into()
  });
  assert!(!stage.paints("Settings"));
  stage.hover("cog");
  after(&mut stage, Duration::from_millis(600));
  assert!(stage.paints("Settings"), "its label shows after the delay");

  stage.hover("find");
  stage.pass();
  assert!(
    stage.paints("Search the page"),
    "the app's tooltip in place of the label"
  );
  assert!(stage.paints("Ctrl+F"));

  // A button with words has none of its own.
  stage.hover("save");
  after(&mut stage, Duration::from_millis(600));
  assert_eq!(
    stage
      .quads()
      .iter()
      .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == "Save"))
      .count(),
    1,
    "only its own label"
  );
}

/// A label 12 of type in a row `width` wide, cut where it does not fit, whose tooltip says `HINT` while it fits.
fn hinted_stage(width: f32, label: &'static str) -> Stage {
  Stage::new(move |ctx| {
    let held = ElementRef::new();
    let words = Text::new(label)
      .id("label")
      .nowrap()
      .text_overflow(TextOverflow::Elipsis)
      .ref_element(held.clone());
    Column::new()
      .size(400.0, 300.0)
      .padding(40.0)
      .child(
        Row::new().width(width).child(
          Tooltip::new(LONG)
            .when_cut(&held)
            .hint(HINT)
            .open_delay(Duration::ZERO)
            .max_width(600.0)
            .mount(ctx, words),
        ),
      )
      .into()
  })
}

const HINT: &str = "Open the release";

#[test]
fn a_hint_is_what_a_tooltip_when_cut_says_while_its_label_fits() {
  let mut stage = hinted_stage(300.0, "Release");
  stage.hover("label");
  stage.pass();
  assert!(stage.paints(HINT), "the hint while the label is whole");
  assert!(!stage.paints(LONG));
}

#[test]
fn a_hinted_tooltip_says_its_text_while_its_label_is_cut() {
  let mut stage = hinted_stage(120.0, LONG);
  stage.hover("label");
  stage.pass();
  assert!(stage.paints(LONG), "the text while the label is cut");
  assert!(!stage.paints(HINT));
}

#[test]
fn a_hung_tooltip_shows_while_the_app_says_and_lets_the_pointer_through() {
  let open = Signal::new(false);
  let held = open.clone();
  let presses = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
  let told = presses.clone();
  let mut stage = Stage::new(move |_| {
    let anchor = ElementRef::new();
    let told = told.clone();
    Column::new()
      .size(400.0, 300.0)
      .padding(100.0)
      .child(
        Button::new("Copy")
          .variant(ButtonVariant::Outline)
          .id("copy")
          .ref_element(anchor.clone()),
      )
      .child(
        Button::new("Under")
          .variant(ButtonVariant::Outline)
          .id("under")
          .on_click(move |_| {
            told.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
          }),
      )
      .child(
        Tooltip::new("Copied to the clipboard")
          .placement(Placement::BottomStart)
          .id("hung")
          .hang(&anchor, held.clone()),
      )
      .into()
  });
  assert!(!stage.paints("Copied to the clipboard"), "closed while the app says so");
  stage.hover("copy");
  after(&mut stage, Duration::from_millis(600));
  assert!(!stage.paints("Copied to the clipboard"), "the pointer does not show it");

  open.set(true);
  stage.pass();
  assert!(stage.paints("Copied to the clipboard"), "the app shows it");
  let (_, cy, _, ch) = stage.bounds("copy");
  let (_, ty, ..) = stage.bounds("hung");
  assert!(ty >= cy + ch, "under the button it hangs from");

  // A press and Escape do not put it away, and what it covers is pressed through it.
  stage.key("Escape");
  stage.click("under");
  assert!(stage.paints("Copied to the clipboard"));
  assert_eq!(presses.load(std::sync::atomic::Ordering::SeqCst), 1);

  open.set(false);
  stage.pass();
  assert!(!stage.paints("Copied to the clipboard"), "the app puts it away");
}

#[test]
fn a_tooltip_over_a_frame_keeps_clear_of_the_frames_top_and_lines_up_with_its_element() {
  let frame = ElementRef::new();
  let held = frame.clone();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(120.0)
      .child(
        Row::new()
          .id("frame")
          .ref_element(held.clone())
          .padding(30.0)
          .child(Row::new().width(60.0))
          .child(
            Tooltip::new("Reaches the agent")
              .placement(Placement::TopStart)
              .over(&held)
              .force_open(Some(true))
              .id("tip")
              .mount(ctx, Button::new("Skill").variant(ButtonVariant::Outline).id("chip")),
          ),
      )
      .into()
  });
  stage.pass();
  let (_, fy, ..) = stage.bounds("frame");
  let (cx, ..) = stage.bounds("chip");
  let (tx, ty, _, th) = stage.bounds("tip");
  assert!(
    (ty + th - fy).abs() < 0.5,
    "its root ends at the frame's top, its room toward it included: {} against {fy}",
    ty + th
  );
  assert!((tx - cx).abs() < 0.5, "at its element's start: {tx} against {cx}");
}
