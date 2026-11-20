//! What a choice of a segmented control or a radio group holds of its own (an id, a count, a state of the theme's,
//! an element ref), and a tab's badge and the reason it is disabled.

use std::time::Duration;

use lurq::{
  app::theme::PaletteColor,
  components::Column,
  core::{ElementRef, Signal},
  time::Instant,
  ui::{Choice, RadioGroup, Recipe, SegmentedControl, SegmentedControlPart, Tab, Tabs},
};

use super::stage::Stage;

fn has(ids: &[String], id: &str) -> bool {
  ids.iter().any(|held| held == id)
}

#[test]
fn a_segment_has_its_own_id_count_state_and_ref() {
  let period = Signal::new("day");
  let anchor = ElementRef::new();
  let (chosen, at) = (period.clone(), anchor.clone());
  let warned = Recipe::new().state("warned", SegmentedControlPart::Segment, |s| {
    s.background(PaletteColor::Destructive)
  });
  let mut stage = Stage::new(move |_| {
    Column::new()
      .child(
        SegmentedControl::new(
          chosen.clone(),
          [
            Choice::new("day", "Day").id("period-day").count(24),
            Choice::new("week", "Week").id("period-week").element_ref(at.clone()),
            Choice::new("year", "Year").id("period-year").state("warned", true),
          ],
        )
        .rules(&warned)
        .id("period"),
      )
      .into()
  });
  stage.pass();
  let ids = stage.ids();
  for id in [
    "period",
    "period-day",
    "period-day-label",
    "period-day-count",
    "period-week-label",
  ] {
    assert!(has(&ids, id), "{id} in {ids:?}");
  }
  assert!(
    !has(&ids, "period-week-count"),
    "a choice with no count has none: {ids:?}"
  );
  assert!(stage.paints("24"), "the count after the label");
  let (x, _, width, _) = stage.bounds("period-week");
  let (ref_x, _, ref_width, _) = anchor.rect();
  assert!(
    (x - ref_x).abs() < 0.5 && (width - ref_width).abs() < 0.5,
    "the ref is the segment's"
  );
  assert_ne!(
    stage.fill_color("period-year"),
    stage.fill_color("period-week"),
    "the state of the choice alone is drawn"
  );
  stage.click("period-week");
  assert_eq!(period.get(), "week");
}

#[test]
fn a_radio_of_a_group_has_its_own_id_count_and_ref() {
  let plan = Signal::new("free");
  let anchor = ElementRef::new();
  let (chosen, at) = (plan.clone(), anchor.clone());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .child(
        RadioGroup::new(
          chosen.clone(),
          [
            Choice::new("free", "Free").id("plan-free"),
            Choice::new("team", "Team")
              .id("plan-team")
              .count(3)
              .element_ref(at.clone()),
          ],
        )
        .id("plan"),
      )
      .into()
  });
  stage.pass();
  let ids = stage.ids();
  for id in ["plan-free", "plan-team", "plan-team-label", "plan-team-count"] {
    assert!(has(&ids, id), "{id} in {ids:?}");
  }
  assert!(stage.paints("3"));
  let (x, _, _, _) = stage.bounds("plan-team");
  assert!((anchor.rect().0 - x).abs() < 0.5, "the ref is the mark's");
  stage.click("plan-team");
  assert_eq!(plan.get(), "team");
}

#[test]
fn a_tab_has_a_badge_after_its_label() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .child(Tabs::new(
        Signal::new(0),
        [
          Tab::new(0, "All").count(24).id("all"),
          Tab::new(1, "Review").badge(3).id("review"),
        ],
      ))
      .into()
  });
  stage.pass();
  let ids = stage.ids();
  assert!(has(&ids, "review-badge") && has(&ids, "all-count"), "{ids:?}");
  assert!(!has(&ids, "all-badge"), "{ids:?}");
  let label = stage.text_quad("Review").expect("the label");
  assert!(
    stage.bounds("review-badge").0 >= label.x + label.width,
    "after the label"
  );
  assert!(stage.fill("review-badge").is_some(), "a pill with a fill");
}

#[test]
fn a_tab_disabled_for_a_reason_says_it_in_its_tooltip_and_cannot_be_selected() {
  let section = Signal::new(0);
  let chosen = section.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(40.0)
      .child(Tabs::new(
        chosen.clone(),
        [
          Tab::new(0, "General").id("general"),
          Tab::new(1, "Insights")
            .disabled_reason("Comes with the next release")
            .id("insights"),
        ],
      ))
      .into()
  });
  stage.pass();
  stage.click("insights");
  assert_eq!(section.get(), 0, "a disabled tab is not selected");
  assert!(!stage.paints("Comes with the next release"));
  stage.hover("insights");
  stage.tree.tick_timers_at(Instant::now() + Duration::from_millis(700));
  stage.pass();
  assert!(
    stage.paints("Comes with the next release"),
    "the reason shows as the tab's tooltip"
  );
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  {
    let said = stage
      .tree
      .get_element_by_id_mut("insights")
      .and_then(|element| element.description("a11y_description"));
    assert_eq!(said.as_deref(), Some("Comes with the next release"));
  }
}
