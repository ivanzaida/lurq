//! The records of a text's objects are kept in step with it: normalized over it, moved by an edit, cut out with
//! a range and put back with one.
use super::*;

const O: char = OBJECT_CHARACTER;

fn object(at: usize, source: &str) -> InlineObject {
  InlineObject::new(at, source)
}

#[test]
fn an_object_is_three_bytes_and_one_character() {
  assert_eq!(O.len_utf8(), 3);
  assert_eq!(object(4, "a.png").range(), 4..7);
}

#[test]
fn objects_are_kept_only_at_their_characters_and_in_the_order_of_the_text() {
  let text = format!("a{O}b{O}");
  let kept = normalize_objects(
    &text,
    vec![
      object(5, "second.png"),
      // No object character at these bytes: in a letter, inside the character, past the end.
      object(0, "letter.png"),
      object(2, "inside.png"),
      object(40, "past.png"),
      object(1, "first.png"),
    ],
  );
  assert_eq!(kept, [object(1, "first.png"), object(5, "second.png")]);
}

#[test]
fn of_two_objects_at_one_byte_the_later_is_kept() {
  let text = format!("{O}");
  let kept = normalize_objects(&text, vec![object(0, "old.png"), object(0, "new.png")]);
  assert_eq!(kept, [object(0, "new.png")]);
}

#[test]
fn an_edit_before_an_object_moves_it_and_one_over_it_takes_it_away() {
  // "ab￼cd￼"
  let objects = [object(2, "one.png"), object(7, "two.png")];
  // Typing a letter at the start moves both.
  assert_eq!(
    splice_objects(&objects, 0..0, 1),
    [object(3, "one.png"), object(8, "two.png")]
  );
  // Deleting the first object leaves the second, three bytes earlier.
  assert_eq!(splice_objects(&objects, 2..5, 0), [object(4, "two.png")]);
  // Replacing "￼cd" with five bytes keeps the last where the difference puts it.
  assert_eq!(splice_objects(&objects, 2..7, 5), [object(7, "two.png")]);
  // An edit after them leaves them be.
  assert_eq!(splice_objects(&objects, 10..10, 4), objects);
}

#[test]
fn a_slice_of_objects_counts_from_its_start_and_is_put_back_from_there() {
  let objects = [object(2, "one.png"), object(7, "two.png"), object(12, "three.png")];
  let slice = slice_objects(&objects, 5..12);
  assert_eq!(slice, [object(2, "two.png")]);
  let placed = place_objects(vec![object(0, "before.png"), object(30, "after.png")], 10, &slice);
  assert_eq!(
    placed,
    [object(0, "before.png"), object(12, "two.png"), object(30, "after.png")]
  );
}

#[test]
fn plain_text_has_the_label_of_an_object_in_its_place() {
  let text = format!("see {O} and {O}.");
  let objects = [object(4, "a.png").label("the lake"), object(12, "b.png")];
  assert_eq!(text_with_labels(&text, &objects), "see the lake and .");
  // A character with no record stays what it is.
  assert_eq!(text_with_labels(&text, &objects[..1]), format!("see the lake and {O}."));
}
