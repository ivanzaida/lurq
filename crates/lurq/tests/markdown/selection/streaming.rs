//! A selection in a reply that streams on: the paragraph renders again with
//! more text, and the selection stays on the same words.

use lurq::components::MarkdownProps;

use super::{page, update};

const FIRST: &str = "The docs say the `messages` table has no attachments yet (`docs/storage.md:490`), so";
const MORE: &str = "The docs say the `messages` table has no attachments yet (`docs/storage.md:490`), so the design \
marks them as not built, but lists a planned `task.attachments` variable.";
const WRAPPED: &str = "The docs say the `messages` table has no attachments yet (`docs/storage.md:490`), so the design \
marks them as not built, but lists a planned `task.attachments` variable, and a second paragraph is on its way.\n\n\
Here it comes, with `code` of its own.";

#[test]
fn a_selection_survives_a_streaming_rerender_of_its_paragraph() {
  let mut page = page(MarkdownProps::new(FIRST), 480.0);
  let (from, to) = page.inks_of("table has no attachments", 0);
  page.drag_inks(from, to);
  assert_eq!(page.selected().as_deref(), Some("table has no attachments"));

  update(&mut page, MarkdownProps::new(MORE));
  assert_eq!(page.selected().as_deref(), Some("table has no attachments"));
  let (from, to) = page.inks_of("table has no attachments", 0);
  page.assert_highlight_covers(from, to);

  update(&mut page, MarkdownProps::new(WRAPPED));
  assert_eq!(page.selected().as_deref(), Some("table has no attachments"));
  let (from, to) = page.inks_of("table has no attachments", 0);
  page.assert_highlight_covers(from, to);
}

#[test]
fn a_selection_can_be_extended_and_replaced_after_a_streaming_rerender() {
  let mut page = page(MarkdownProps::new(FIRST), 480.0);
  let (from, to) = page.inks_of("messages", 0);
  page.drag_inks(from, to);
  update(&mut page, MarkdownProps::new(WRAPPED));

  let (_, planned) = page.inks_of("planned", 0);
  let after = page.after(planned);
  page.shift_click(after.0, after.1);
  let (from, _) = page.inks_of("messages", 0);
  assert_eq!(page.selected().as_deref(), Some(&*page.text_between(from, planned)));

  let (from, to) = page.inks_of("Here it comes", 0);
  page.drag_inks(from, to);
  assert_eq!(page.selected().as_deref(), Some("Here it comes"));
  page.assert_highlight_covers(from, to);
}

#[test]
fn a_drag_in_progress_follows_the_glyphs_after_a_streaming_rerender() {
  let mut page = page(MarkdownProps::new(FIRST), 480.0);
  let (from, _) = page.inks_of("table", 0);
  let before = page.before(from);
  page.press(before.0, before.1);
  update(&mut page, MarkdownProps::new(MORE));
  let (_, to) = page.inks_of("planned", 0);
  let after = page.after(to);
  page.tree.mouse_move(after.0, after.1);
  page.release(after.0, after.1);
  let (from, _) = page.inks_of("table", 0);
  assert_eq!(page.selected().as_deref(), Some(&*page.text_between(from, to)));
}
