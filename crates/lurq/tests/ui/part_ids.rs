//! The ids a component given one gives the nodes of its parts: `x-<part>`, numbered where a part is on several
//! nodes, an item's own id over the component's, a component built inside another named after the part it stands
//! for, none without an id, and the same ids on every render.

use lurq::{
  components::Column,
  core::{Date, Signal},
  ui::{Accordion, AccordionItem, Badge, Button, Callout, DataList, DatePicker, Progress, icons},
};

use super::stage::Stage;

fn ids_of(scene: impl Fn() -> lurq::node::Element + Send + Sync + 'static) -> Vec<String> {
  let mut stage = Stage::new(move |_| Column::new().child(scene()).into());
  stage.pass();
  stage.ids()
}

#[test]
fn a_component_with_an_id_names_its_parts_after_it() {
  let ids = ids_of(|| Badge::new("Online").dot(true).icon(icons::CHECK).id("status").into());
  assert_eq!(ids, ["status", "status-dot", "status-icon", "status-label"]);

  let ids = ids_of(|| Callout::new("It is close.").title("Heads up").id("note").into());
  assert!(ids.starts_with(&["note".to_string()]), "{ids:?}");
  for part in ["note-icon", "note-text", "note-title", "note-label"] {
    assert!(ids.iter().any(|id| id == part), "{part} in {ids:?}");
  }

  let ids = ids_of(|| Progress::new(0.4).id("upload").into());
  assert!(ids.iter().any(|id| id == "upload-fill"), "{ids:?}");
}

#[test]
fn a_part_on_several_nodes_is_numbered_in_build_order() {
  let ids = ids_of(|| {
    DataList::new()
      .text("Version", "0.63.0")
      .text("Renderer", "wgpu")
      .text("License", "MIT")
      .id("about")
      .into()
  });
  assert_eq!(
    ids,
    [
      "about",
      "about-term",
      "about-value",
      "about-term-2",
      "about-value-2",
      "about-term-3",
      "about-value-3"
    ],
    "the lined items and the last, resolved apart, are counted together"
  );
}

#[test]
fn an_item_with_an_id_keeps_it_and_names_its_parts_after_it() {
  let ids = ids_of(|| {
    let open = Signal::new(Some("gpu"));
    Accordion::optional(
      open,
      [
        AccordionItem::text("gpu", "Is lurq GPU-accelerated?", "Yes."),
        AccordionItem::text("theme", "Can I bring my own theme?", "Yes.").id("theme"),
      ],
    )
    .id("faq")
    .into()
  });
  for id in [
    "faq",
    "faq-trigger",
    "faq-label",
    "faq-body",
    "theme",
    "theme-label",
    "theme-chevron",
  ] {
    assert!(ids.iter().any(|held| held == id), "{id} in {ids:?}");
  }
  assert!(
    !ids.iter().any(|id| id == "faq-label-2"),
    "the item with an id names its label after itself: {ids:?}"
  );
}

#[test]
fn a_component_built_inside_another_is_named_after_the_part_it_stands_for() {
  let ids = ids_of(|| Button::new("Saving…").loading(true).id("save").into());
  assert!(ids.iter().any(|id| id == "save-spinner"), "{ids:?}");
  assert!(
    ids.iter().any(|id| id == "save-spinner-indicator"),
    "the spinner names its own parts after its id: {ids:?}"
  );
  assert_eq!(
    ids
      .iter()
      .filter(|id| id.starts_with("save-spinner") && !id.contains("indicator"))
      .count(),
    1,
    "the node that holds the spinner takes no id of its own: {ids:?}"
  );
}

#[test]
fn a_component_with_no_id_gives_its_parts_none() {
  let ids = ids_of(|| {
    Column::new()
      .child(Badge::new("Online").dot(true).icon(icons::CHECK))
      .child(DataList::new().text("Version", "0.63.0"))
      .child(Button::new("Saving…").loading(true))
      .into()
  });
  assert!(ids.is_empty(), "{ids:?}");
}

#[test]
fn the_ids_are_the_same_on_every_render() {
  let label = Signal::new("Online".to_string());
  let read = label.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .child(Badge::new(&read.get()).dot(true).id("status"))
      .child(
        DataList::new()
          .text("Version", &read.get())
          .text("License", "MIT")
          .id("about"),
      )
      .child(Button::new(&read.get()).loading(true).id("save"))
      .into()
  });
  stage.pass();
  let first = stage.ids();
  label.set("Away".to_string());
  stage.pass();
  assert!(stage.paints("Away"), "rendered again");
  assert_eq!(stage.ids(), first);
}

#[test]
fn a_part_with_a_key_of_its_own_is_named_by_it_and_a_component_inside_by_its_own_name() {
  let day = |number| Date::new(2026, 11, number).unwrap();
  let value = Signal::new(Some(day(20)));
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 500.0)
      .child(
        DatePicker::new(value.clone())
          .today(day(9))
          .inline(true)
          .id("due")
          .mount(ctx),
      )
      .into()
  });
  stage.pass();
  let ids = stage.ids();
  let has = |id: &str| ids.iter().any(|held| held == id);
  for id in [
    "due",
    "due-trigger",
    "due-trigger_icon",
    "due-panel",
    "due-calendar",
    "due-calendar-title",
    "due-calendar-nav-previous",
    "due-calendar-nav-next",
    "due-calendar-weekday-mon",
    "due-calendar-weekday-sun",
    "due-calendar-day-2026-11-20",
    "due-calendar-day-2026-11-01",
  ] {
    assert!(has(id), "{id} in {ids:?}");
  }
  assert!(
    !ids.iter().any(|id| id.starts_with("due-panel-calendar")),
    "the calendar is named by its own name, not after the panel it is in: {ids:?}"
  );
  let mut unique = ids.clone();
  unique.sort();
  unique.dedup();
  assert_eq!(unique.len(), ids.len(), "no id twice: {ids:?}");
}
