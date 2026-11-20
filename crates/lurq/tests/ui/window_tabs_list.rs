//! Window tabs that leave the strip for a list, as Orchester's title bar has them: a gap between tabs that the
//! fitting counts, so a tab that left comes back when the strip has room for it again; separators that stand the
//! same whether a tab is in the strip or not; and a tab as wide as what it shows, up to a cap.

use lurq::{
  components::{Column, Row},
  core::Signal,
  ui::{Recipe, WindowTab, WindowTabs, WindowTabsOverflow, WindowTabsPart},
};

use super::{
  stage::Stage,
  window_tabs::{close_to, wide},
};

/// The rules of a strip whose tabs are 2 apart, and 2 before the add button.
fn gapped(separators: bool) -> Recipe {
  let rules = Recipe::new()
    .base(WindowTabsPart::List, |s| s.gap(2.0))
    .base(WindowTabsPart::Root, |s| s.gap(2.0));
  if separators {
    rules
  } else {
    rules.base(WindowTabsPart::Separator, |s| s.width(0.0).height(0.0))
  }
}

/// `count` tabs of ids `t0`.. in a room as wide as `room` says, in the list mode with `rules`, the first selected.
fn roomy(count: usize, room: Signal<f32>, rules: Recipe) -> Stage {
  wide(move |ctx| {
    let tabs = WindowTabs::controlled(
      0,
      (0..count).map(|index| WindowTab::new(index, &format!("{index}.rs")).id(format!("t{index}"))),
      |_| {},
    )
    .id("strip")
    .overflow(WindowTabsOverflow::List)
    .rules(&rules)
    .on_add(|| {})
    .mount(ctx);
    Column::new()
      .size(800.0, 300.0)
      .child(Row::new().width(room.get()).child(tabs))
      .into()
  })
}

/// The tabs of `count` that are in the strip, by index.
fn in_strip(stage: &mut Stage, count: usize) -> Vec<usize> {
  (0..count)
    .filter(|index| stage.bounds(&format!("t{index}")).2 > 0.0)
    .collect()
}

/// Lays the stage out again, until the tabs that leave and come back are settled.
fn settle(stage: &mut Stage) {
  for _ in 0..3 {
    stage.pass();
  }
}

#[test]
fn tabs_keep_the_lists_gap_and_one_that_left_comes_back_when_the_room_is_there_again() {
  for separators in [false, true] {
    // Three tabs of 128, 2 apart, then 2 before the add button of 30: all three fit in 420 with no separators. A
    // separator of 1 stands between the two plain tabs, with its gap, and one before the add button.
    let all = if separators { 420.0 + 1.0 + 2.0 + 1.0 + 2.0 } else { 420.0 };
    let room = Signal::new(all);
    let mut stage = roomy(3, room.clone(), gapped(separators));
    assert_eq!(in_strip(&mut stage, 3), [0, 1, 2], "all fit: {separators}");
    let (x0, ..) = stage.bounds("t0");
    let (x1, ..) = stage.bounds("t1");
    assert!(close_to(x1, x0 + 128.0 + 2.0), "2 apart: {x0} {x1}");
    let (_, _, strip, _) = stage.bounds("strip");
    assert!(close_to(strip, all), "the strip as wide as they are with their gaps: {strip}");

    room.set(all - 1.0);
    settle(&mut stage);
    assert_eq!(in_strip(&mut stage, 3), [0, 1], "the last left: {separators}");
    let (more_x, ..) = stage.bounds("strip-more");
    let (x1, _, w1, _) = stage.bounds("t1");
    assert!(close_to(more_x, x1 + w1 + 2.0), "the More button 2 after the tabs: {more_x}");

    room.set(all);
    settle(&mut stage);
    assert_eq!(in_strip(&mut stage, 3), [0, 1, 2], "back with the room: {separators}");
    assert_eq!(stage.bounds("strip-more").2, 0.0, "no More button");
  }
}

#[test]
fn a_tab_that_fits_its_content_is_as_wide_as_what_it_shows_up_to_its_cap() {
  let label = |stage: &Stage, text: &str| stage.text_quad(text).map(|quad| quad.width);
  for overflow in [WindowTabsOverflow::Scroll, WindowTabsOverflow::List] {
    let mut stage = wide(move |ctx| {
      let tabs = WindowTabs::controlled(
        0,
        [
          WindowTab::new(0, "a.rs").id("short").fit_content(240.0),
          WindowTab::new(1, "a_label_that_is_far_longer_than_its_cap.rs")
            .id("long")
            .fit_content(160.0),
          WindowTab::new(2, "b.rs").id("plain"),
        ],
        |_| {},
      )
      .id("strip")
      .overflow(overflow)
      .rules(&Recipe::new().base(WindowTabsPart::Tab, |s| s.min_width(0.0)));
      Row::new().child(tabs.mount(ctx)).into()
    });
    let (short_x, _, short, _) = stage.bounds("short");
    let close = stage.bounds("short-close");
    let text = label(&stage, "a.rs").expect("the label is painted");
    // Padded 10, the label, 8, the close slot of 18, padded 6.
    assert!(
      close_to(short, 10.0 + text + 8.0 + 18.0 + 6.0),
      "as wide as what it shows: {short} {text} {overflow:?}"
    );
    assert!(close_to(close.0 + close.2, short_x + short - 6.0));
    assert_eq!(stage.bounds("long").2, 160.0, "the cap: {overflow:?}");
    assert!(
      stage
        .cut_text_quad("a_label_that_is_far_longer_than_its_cap.rs")
        .is_some(),
      "cut with an ellipsis"
    );
    assert_eq!(stage.bounds("plain").2, 128.0, "another keeps the recipe's");
    let (_, _, strip, _) = stage.bounds("strip");
    assert!(strip >= short + 160.0 + 128.0 - 0.5, "{strip}");
  }
}
