//! A selection across the objects of a rich text (`Text::object`): its carets meet the words past an object, and
//! a copy gives what each object copies (`copy_text`), never its object character.

use super::{Tree, selected_selectable_text};
use crate::{
  app::{App, events::MouseButton},
  components::{Column, Text},
  layout::{quad::RichTextSpan, text_style::TextStyle},
};

fn style() -> TextStyle {
  TextStyle {
    font_size: 14.0,
    line_height: 1.5,
    ..TextStyle::default()
  }
}

fn sentence(copied: Option<&str>) -> Text {
  let mut object = Column::new().size(60.0, 12.0).id("object");
  if let Some(copied) = copied {
    object = object.copy_text(copied);
  }
  Text::rich([
    RichTextSpan::new("See ", style()),
    RichTextSpan::object(style()),
    RichTextSpan::new(" today.", style()),
  ])
  .selectable(true)
  .object(object)
  .id("sentence")
}

/// Drags across the sentence from `from` to `to`, its x in the sentence's box, and gives what a copy would.
fn drag(tree: &mut Tree, app: &mut App, from: f32, to: f32) -> Option<String> {
  tree.pass_headless(app);
  let size = tree.last_layout().expect("laid out").size;
  let y = size.height / 2.0;
  tree.mouse_down(from, y, MouseButton::Left);
  tree.mouse_move(to, y);
  tree.mouse_up(to, y, MouseButton::Left);
  tree.pass_headless(app);
  selected_selectable_text(tree.root.as_ref()?, tree.last_layout.as_ref()?)
}

#[test]
fn a_copy_across_an_object_gives_what_it_copies_and_never_its_character() {
  let (mut app, mut tree) = (App::new(), Tree::new());
  tree.set_root(sentence(Some("[Layout](https://lurq.dev/docs/layout)")));
  tree.pass_headless(&mut app);
  let width = tree.last_layout().expect("laid out").size.width;
  assert_eq!(
    drag(&mut tree, &mut app, 1.0, width - 1.0).as_deref(),
    Some("See [Layout](https://lurq.dev/docs/layout) today.")
  );

  // An object that says nothing copies nothing.
  let (mut app, mut tree) = (App::new(), Tree::new());
  tree.set_root(sentence(None));
  tree.pass_headless(&mut app);
  let width = tree.last_layout().expect("laid out").size.width;
  let copied = drag(&mut tree, &mut app, 1.0, width - 1.0).unwrap_or_default();
  assert_eq!(copied, "See  today.");
  assert!(!copied.contains(crate::components::OBJECT_CHARACTER));
}

#[test]
fn the_carets_past_an_object_are_where_its_words_are_drawn() {
  // From the start to just past the object: the words before it and the object, and not one character of the
  // words after it, which a caret of the narrow object character would have counted.
  let (mut app, mut tree) = (App::new(), Tree::new());
  tree.set_root(sentence(Some("[object]")));
  tree.pass_headless(&mut app);
  let object = tree
    .get_element_by_id_mut("object")
    .and_then(|element| element.bounds())
    .expect("the object is laid out");
  let sentence = tree
    .get_element_by_id_mut("sentence")
    .and_then(|element| element.bounds())
    .expect("the sentence is laid out");
  let past = object.x + object.width - sentence.x + 0.5;
  assert_eq!(drag(&mut tree, &mut app, 1.0, past).as_deref(), Some("See [object]"));
}
