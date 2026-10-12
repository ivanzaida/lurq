//! The exact private editing operations called by runtime clipboard dispatch;
//! these tests do not read or replace the user's system clipboard.
use super::*;
use crate::components::{InputToken, InputTokenStyle, InputTokens};

#[test]
fn input_tokens_clipboard_payload_cut_paste_and_undo_remain_raw_source() {
  let source = Signal::new("left {{task.title}} right".to_owned());
  let state = TextInputState::new(source.clone());
  state.set_tokens(
    InputTokens::new(
      &source.get(),
      vec![InputToken::new(
        5..19,
        InputTokenStyle::new(
          crate::node::color::Color::new(1, 2, 3, 255),
          crate::node::color::Color::new(4, 5, 6, 255),
        ),
      )],
    )
    .unwrap(),
  );
  assert!(state.controller_select(5, 19));
  assert_eq!(state.selected_text().as_deref(), Some("{{task.title}}"));
  let event = KeyboardEvent::new("x", "KeyX", false, true, false, false, crate::core::NodeId::UNASSIGNED);
  let clipboard = state.cut_selection(&event).unwrap();
  assert_eq!(clipboard, "{{task.title}}");
  assert_eq!(source.get(), "left  right");
  assert!(state.insert(&clipboard, &event));
  assert_eq!(source.get(), "left {{task.title}} right");
  assert!(state.undo(&event));
  assert_eq!(source.get(), "left  right");
  assert!(state.undo(&event));
  assert_eq!(source.get(), "left {{task.title}} right");
  assert_eq!(state.selected_text().as_deref(), Some("{{task.title}}"));
}
