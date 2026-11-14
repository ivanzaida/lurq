//! A collapsible bound to a signal, what its handlers are told, and the icon of its chevron. Its header against the
//! board, the keyboard and a theme's other anatomy are in the accordion's tests, beside the accordion's items.

use std::sync::{Arc, Mutex};

use lurq::{
  components::Column,
  core::Signal,
  layout::quad::QuadContent,
  ui::{Collapsible, CollapsiblePart, icons},
};

use super::stage::Stage;

const CHEVRONS_VERTICAL: &str = "\u{E01D}";

const BODY: &str = "lurq, lurq_macros, demo";

/// How many icons are painted: texts that are a glyph of the icon font alone.
fn icons_painted(stage: &Stage) -> usize {
  let glyph = |c: char| ('\u{E000}'..='\u{F8FF}').contains(&c);
  stage
    .quads()
    .iter()
    .filter(
      |quad| matches!(&quad.content, QuadContent::Text { text, .. } if !text.is_empty() && text.chars().all(glyph)),
    )
    .count()
}

#[test]
fn a_bound_collapsible_sets_its_signal_and_tells_its_handlers_after_it() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let (tell, open) = (told.clone(), Signal::new(false));
  let read = open.clone();
  let mut stage = Stage::new(move |_| {
    let (tell, seen) = (tell.clone(), read.clone());
    Column::new()
      .padding(8.0)
      .child(
        // The handler is given before the signal, and is still told once the signal is set.
        Collapsible::text("3 crates in this workspace", BODY)
          .on_change(move |next| tell.lock().unwrap().push((next, seen.get_untracked())))
          .bind(read.clone())
          .id("crates")
          .style(CollapsiblePart::Root, |s| s.width(280.0)),
      )
      .into()
  });
  assert!(!stage.paints(BODY));
  stage.click("crates");
  assert!(open.get(), "a press sets the signal");
  assert!(stage.paints(BODY));
  // The handler is told the new state, and the signal has it already.
  assert_eq!(*told.lock().unwrap(), [(true, true)]);
  stage.key("Enter");
  assert!(!open.get());
  assert_eq!(told.lock().unwrap()[1..], [(false, false)]);
  // The app sets the signal: it opens, and nothing is told.
  open.set(true);
  stage.pass();
  assert!(stage.paints(BODY));
  assert_eq!(told.lock().unwrap().len(), 2);
}

#[test]
fn the_chevron_has_the_icon_it_is_given_and_none_with_none() {
  let shown = |open: bool, collapsible: fn(Collapsible) -> Collapsible| {
    let stage = Stage::new(move |_| {
      Column::new()
        .padding(8.0)
        .child(collapsible(Collapsible::text("Details", "More").open(open)))
        .into()
    });
    let painted = icons_painted(&stage);
    (stage.paints(CHEVRONS_VERTICAL), painted)
  };
  // Unless told otherwise, `chevrons-vertical`, open or closed.
  assert_eq!(shown(false, |c| c), (true, 1));
  assert_eq!(shown(true, |c| c), (true, 1));
  // Another icon in its place, and one for the open form.
  assert_eq!(
    shown(false, |c| c.icon(icons::CHEVRON_DOWN).open_icon(icons::CHEVRON_UP)),
    (false, 1)
  );
  assert_eq!(
    shown(true, |c| c.icon(icons::CHEVRON_DOWN).open_icon(icons::CHEVRON_UP)),
    (false, 1)
  );
  // The open form has the closed one's icon where it is given none of its own.
  assert_eq!(shown(true, |c| c.icon(icons::CHEVRON_DOWN)), (false, 1));
  // None: the header has no chevron.
  assert_eq!(shown(false, |c| c.icon(None)), (false, 0));
  assert_eq!(shown(true, |c| c.open_icon(None)), (false, 0));
  assert_eq!(shown(false, |c| c.open_icon(None)), (true, 1));
}
