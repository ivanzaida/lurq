//! Objects of a styled input: a box in a row of its text, one thing to the caret, a child node placed on its box,
//! kept through the undo history and the clipboard.
use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App, ClipboardReader, ClipboardWriter, Tree,
    component::Component,
    ctx::Ctx,
    events::{ClipboardContent, MouseButton, ScrollPhase},
  },
  components::{
    Column, InlineObject, OBJECT_CHARACTER, ParagraphKind, ParagraphStyle, ParagraphStyles, Rect, RichText, StyledSpan,
    TextInput, TextInputController,
  },
  core::Signal,
  layout::text_style::TextStyle,
  node::{Element, color::Color},
};

use crate::support::{RectSnapshot, RenderSnapshot, pointer_click, render_pass_with_app};

const ID: &str = "document";
const O: char = OBJECT_CHARACTER;
const RED: Color = Color::new(200, 30, 60, 255);
const GREEN: Color = Color::new(30, 160, 60, 255);
const CARET: Color = Color::new(255, 11, 33, 255);
const SELECTION: Color = Color::new(7, 77, 177, 255);
/// The line of the input's type: 14 at 1.5.
const LINE: f32 = 21.0;
const INDENT: f32 = 30.0;

/// An object the tests draw as a box of `width` by `height` in red, or in green where `green`.
fn swatch(at: usize, width: u32, height: u32, green: bool) -> InlineObject {
  InlineObject::new(at, format!("{width}x{height}"))
    .label(if green { "green" } else { "red" })
    .width(width)
}

/// The box an object is drawn as: of the size its source says, or as wide as the document has it.
fn drawn(object: &InlineObject) -> Element {
  let (width, height) = object.source.split_once('x').expect("a size");
  let color = if object.label.as_ref() == "green" { GREEN } else { RED };
  let width = object.width.map_or(width.parse::<f32>().unwrap(), |width| width as f32);
  Rect::new(width, height.parse::<f32>().unwrap())
    .background(color)
    .into()
}

#[derive(Clone)]
struct Props {
  value: Signal<String>,
  spans: Signal<Vec<StyledSpan>>,
  kinds: Signal<Vec<ParagraphKind>>,
  objects: Signal<Vec<InlineObject>>,
  controller: TextInputController,
  width: Signal<f32>,
  rows: Signal<usize>,
}

impl PartialEq for Props {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Props {}

struct Scene;

impl Component for Scene {
  type Props = Props;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Props>().clone();
    let styles = ParagraphStyles::new().with(ParagraphKind::Quote, ParagraphStyle::new().indent(INDENT));
    let input = TextInput::styled(
      props.value.clone(),
      TextStyle {
        font_size: 14.0,
        line_height: 1.5,
        ..TextStyle::default()
      },
    )
    .id(ID)
    .spans(props.spans.clone())
    .paragraphs(props.kinds.clone(), styles)
    .objects(props.objects.clone(), drawn)
    .resizable_objects(true)
    .rich_clipboard(true)
    .controller(props.controller.clone())
    .caret_color(CARET)
    .selection_color(SELECTION)
    .multiline()
    .rows(1, props.rows.get())
    .width(props.width.get());
    Column::new().size(800.0, 600.0).child(input)
  }
}

/// A focused multiline styled input with objects.
struct Document {
  tree: Tree,
  app: App,
  props: Props,
}

impl Document {
  fn new(text: &str, objects: Vec<InlineObject>) -> Self {
    Self::with(text, objects, Vec::new())
  }

  fn with(text: &str, objects: Vec<InlineObject>, kinds: Vec<ParagraphKind>) -> Self {
    let props = Props {
      value: Signal::new(text.to_owned()),
      spans: Signal::new(Vec::new()),
      kinds: Signal::new(kinds),
      objects: Signal::new(objects),
      controller: TextInputController::new(),
      width: Signal::new(400.0),
      rows: Signal::new(40),
    };
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.resize(800, 600);
    tree.mount_root::<Scene>(&mut app, props.clone());
    let mut document = Self { tree, app, props };
    document.render();
    document.tree.get_element_by_id_mut(ID).unwrap().focus();
    document.render();
    document
  }

  fn render(&mut self) -> RenderSnapshot {
    render_pass_with_app(&mut self.tree, &mut self.app)
  }

  /// The boxes drawn in `color`, in the order of the page.
  fn boxes(&mut self, color: Color) -> Vec<RectSnapshot> {
    let mut boxes: Vec<_> = self
      .render()
      .rects
      .into_iter()
      .filter(|rect| rect.color == color)
      .collect();
    boxes.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    boxes
  }

  fn the_box(&mut self, color: Color) -> RectSnapshot {
    let boxes = self.boxes(color);
    assert_eq!(boxes.len(), 1, "one box of {color:?}");
    boxes[0].clone()
  }

  /// The caret as it is drawn with the caret at `byte`.
  fn caret_at(&mut self, byte: usize) -> RectSnapshot {
    self.props.controller.select(byte, byte).unwrap();
    self.the_box(CARET)
  }

  fn key(&mut self, key: &str) {
    self.tree.key_down(key.into(), key.into(), false, false, false);
    self.render();
  }

  fn shift_key(&mut self, key: &str) {
    self.tree.key_down(key.into(), key.into(), true, false, false);
    self.render();
  }

  /// Ctrl+key, or Cmd+key on macOS and iOS.
  fn shortcut(&mut self, key: &str, code: &str, shift: bool) {
    let (ctrl, meta) = (cfg!(not(target_vendor = "apple")), cfg!(target_vendor = "apple"));
    self
      .tree
      .key_down_with_meta(key.into(), code.into(), shift, ctrl, false, meta);
    self.render();
  }

  fn type_text(&mut self, text: &str) {
    for ch in text.chars() {
      self.tree.key_down(ch.to_string(), "Key".into(), false, false, false);
    }
    self.render();
  }

  fn selection(&self) -> std::ops::Range<usize> {
    self.props.controller.selection().unwrap().range()
  }

  fn caret(&self) -> usize {
    self.props.controller.selection().unwrap().caret
  }

  fn text(&self) -> String {
    self.props.value.get()
  }

  fn objects(&self) -> Vec<InlineObject> {
    self.props.objects.get()
  }

  /// The left edge of the input's text.
  fn left(&mut self) -> f32 {
    self.tree.get_element_by_id_mut(ID).unwrap().bounds().unwrap().x
  }
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.6
}

#[test]
fn styled_an_object_is_a_box_of_its_size_between_the_carets_either_side_of_it() {
  let mut document = Document::new(&format!("ab{O}cd"), vec![swatch(2, 40, 30, false)]);
  let room = document.the_box(RED);
  assert!(close(room.width, 40.0) && close(room.height, 30.0), "{room:?}");
  let before = document.caret_at(2);
  let after = document.caret_at(5);
  // The caret before it stands at its start, the one after it at its end: both beside it, as high as a line of
  // the text and not as the box.
  assert!(
    close(before.x + before.width, room.x) || close(before.x, room.x),
    "{before:?} {room:?}"
  );
  assert!(close(after.x, room.x + room.width), "{after:?} {room:?}");
  // The caret is drawn over the box, which stands where it does.
  assert!(before.order > room.order && after.order > room.order);
  let plain = Document::new("abcd", Vec::new()).caret_at(2);
  assert!(close(before.height, plain.height) && close(after.height, plain.height));
  // The glyphs after it start past it.
  let end = document.caret_at(7);
  assert!(end.x > room.x + room.width + 5.0);
}

#[test]
fn styled_a_row_is_as_high_as_its_tallest_object_which_stands_on_the_baseline() {
  let text = format!("ab{O}cd{O}\nsecond");
  let mut document = Document::new(&text, vec![swatch(2, 40, 60, false), swatch(7, 20, 30, true)]);
  let (tall, short) = (document.the_box(RED), document.the_box(GREEN));
  // Both stand on one line: the baseline of the row.
  assert!(
    close(tall.y + tall.height, short.y + short.height),
    "{tall:?} {short:?}"
  );
  let first = document.caret_at(0);
  let second = document.caret_at(text.len());
  // The text keeps its line at the foot of the row, and the next row starts under it.
  let plain_pitch = {
    let mut plain = Document::new("abcd\nsecond", Vec::new());
    plain.caret_at(11).y - plain.caret_at(0).y
  };
  assert!(
    close(second.y - first.y, plain_pitch),
    "the line of the text is at the foot of its row"
  );
  // The row is higher than a line by what the tall box rises over it, and the box is inside the row.
  assert!(first.y > tall.y, "the caret stands lower than the top of the box");
  assert!(tall.y + tall.height > first.y && tall.y + tall.height <= first.y + first.height + 0.6);
  // An object no higher than the text's ascent leaves the row a line high.
  let mut low = Document::new(&format!("ab{O}cd\nsecond"), vec![swatch(2, 40, 8, false)]);
  let pitch = low.caret_at(10).y - low.caret_at(0).y;
  assert!(close(pitch, LINE), "{pitch}");
}

#[test]
fn styled_an_object_that_does_not_fit_the_rest_of_its_row_goes_to_the_next() {
  let words = "one two three four five six seven eight nine ten eleven ";
  let mut document = Document::new(&format!("{words}{O}"), vec![swatch(words.len(), 200, 40, false)]);
  let left = document.left();
  let room = document.the_box(RED);
  let first = document.caret_at(0);
  assert!(close(room.x, left), "at the start of its row: {room:?}");
  assert!(room.y >= first.y + first.height - 0.6, "under the first row");
  // In a room that holds both they are on one row.
  document.props.width.set(700.0);
  let room = document.the_box(RED);
  assert!(room.x > left + 200.0, "{room:?}");
}

#[test]
fn styled_an_object_wider_than_its_row_is_as_wide_as_the_row_and_keeps_its_shape() {
  let mut document = Document::new(&format!("a\n{O}\nz"), vec![swatch(2, 1000, 500, false)]);
  // What is drawn in the box is the element's own business; the box is the room it is cut to.
  let room = document.the_box(RED).clip;
  // The row keeps the room of a caret after it: the caret there is inside the input.
  assert!(close(room.width, 398.0) && close(room.height, 199.0), "{room:?}");
  let (above, below) = (document.caret_at(0), document.caret_at(6));
  assert!(below.y - above.y > 199.0 && below.y - above.y < 199.0 + 2.0 * LINE);
  let after = document.caret_at(5);
  let left = document.left();
  assert!(
    after.x >= room.x + room.width - 0.6 && after.x + after.width <= left + 400.0 + 0.6,
    "{after:?}"
  );
  // In a paragraph that keeps room before its text, the row is what is left.
  let mut quoted = Document::with(
    &format!("a\n{O}"),
    vec![swatch(2, 1000, 500, false)],
    vec![ParagraphKind::Body, ParagraphKind::Quote],
  );
  let left = quoted.left();
  let room = quoted.the_box(RED).clip;
  assert!(
    close(room.x, left + INDENT) && close(room.width, 398.0 - INDENT),
    "{room:?}"
  );
  assert!(close(room.height, (398.0 - INDENT) / 2.0), "{room:?}");
}

#[test]
fn styled_the_arrows_go_past_an_object_as_past_one_character() {
  let mut document = Document::new(&format!("ab{O}cd"), vec![swatch(2, 40, 30, false)]);
  document.props.controller.select(2, 2).unwrap();
  document.key("ArrowRight");
  assert_eq!(document.caret(), 5);
  document.key("ArrowLeft");
  assert_eq!(document.caret(), 2);
  document.shift_key("ArrowRight");
  assert_eq!(document.selection(), 2..5);
}

#[test]
fn styled_a_press_on_an_object_selects_it_and_a_press_beside_it_puts_the_caret_there() {
  let mut document = Document::new(&format!("ab{O}cd"), vec![swatch(2, 40, 30, false)]);
  let room = document.the_box(RED);
  pointer_click(&mut document.tree, room.x + 20.0, room.y + 10.0, MouseButton::Left);
  assert_eq!(document.selection(), 2..5, "the object, all of it");
  // The veil of a selected object is drawn over it, in the selection's color.
  let veil = document
    .render()
    .rects
    .into_iter()
    .find(|rect| rect.color.r() == SELECTION.r() && rect.color.a() < 255 && close(rect.height, 30.0));
  let veil = veil.expect("a veil over the selected object");
  assert!(close(veil.x, room.x) && close(veil.y, room.y) && close(veil.width, 40.0));
  // The veil is all of it: the highlight of text has no band behind a box.
  assert!(document.boxes(SELECTION).is_empty());
  // A selection over the letters round it highlights them and veils it.
  document.props.controller.select(1, 6).unwrap();
  let bands = document.boxes(SELECTION);
  assert_eq!(
    bands.len(),
    2,
    "the letter before it and the letter after it: {bands:?}"
  );
  assert!(bands[0].x + bands[0].width <= room.x + 0.6 && bands[1].x >= room.x + 40.0 - 0.6);
  // A press right of it, on its row, is a caret after it.
  pointer_click(&mut document.tree, room.x + 41.0, room.y + 25.0, MouseButton::Left);
  assert_eq!(document.selection(), 5..5);
}

#[test]
fn styled_backspace_after_an_object_selects_it_and_the_next_deletes_it_and_undo_brings_it_back() {
  let picture = swatch(2, 40, 30, false);
  let mut document = Document::new(&format!("ab{O}cd"), vec![picture.clone()]);
  document.props.controller.select(5, 5).unwrap();
  document.key("Backspace");
  assert_eq!(document.selection(), 2..5);
  assert_eq!(
    document.text(),
    format!("ab{O}cd"),
    "the first Backspace deletes nothing"
  );
  document.key("Backspace");
  assert_eq!(document.text(), "abcd");
  assert!(document.objects().is_empty(), "its record went with its character");
  assert!(document.boxes(RED).is_empty(), "and its child is gone");
  document.shortcut("z", "KeyZ", false);
  assert_eq!(document.text(), format!("ab{O}cd"));
  assert_eq!(document.objects(), [picture], "with its source, its label and its size");
  assert_eq!(document.boxes(RED).len(), 1);
  document.shortcut("z", "KeyZ", true);
  assert_eq!(document.text(), "abcd");
  assert!(document.objects().is_empty());
  // Delete before one works the same from the other side.
  document.shortcut("z", "KeyZ", false);
  document.props.controller.select(2, 2).unwrap();
  document.key("Delete");
  assert_eq!(document.selection(), 2..5);
  document.key("Delete");
  assert_eq!(document.text(), "abcd");
}

#[test]
fn styled_an_edit_before_an_object_moves_its_record_and_its_child() {
  let mut document = Document::new(
    &format!("ab{O}cd{O}"),
    vec![swatch(2, 40, 30, false), swatch(7, 20, 20, true)],
  );
  let (red, green) = (document.the_box(RED), document.the_box(GREEN));
  document.props.controller.select(0, 0).unwrap();
  document.type_text("xyz");
  assert_eq!(document.text(), format!("xyzab{O}cd{O}"));
  assert_eq!(document.objects(), [swatch(5, 40, 30, false), swatch(10, 20, 20, true)]);
  let (moved_red, moved_green) = (document.the_box(RED), document.the_box(GREEN));
  let by = moved_red.x - red.x;
  assert!(by > 10.0, "the box moved with the text: {by}");
  assert!(close(moved_green.x - green.x, by));
  // An edit between the two moves the second alone.
  document.props.controller.select(8, 8).unwrap();
  document.type_text("!");
  assert_eq!(document.objects(), [swatch(5, 40, 30, false), swatch(11, 20, 20, true)]);
  assert!(close(document.the_box(RED).x, moved_red.x));
  assert!(document.the_box(GREEN).x > moved_green.x);
  // A selection over the first and the letters round it, typed over, leaves the second where the text puts it.
  document.props.controller.select(4, 9).unwrap();
  document.type_text("-");
  assert_eq!(document.text(), format!("xyza-cd{O}"));
  assert_eq!(document.objects(), [swatch(7, 20, 20, true)]);
  assert!(document.boxes(RED).is_empty());
  assert_eq!(document.boxes(GREEN).len(), 1);
}

#[test]
fn styled_the_child_of_a_deleted_object_is_gone_before_the_children_are_built_again() {
  // An app that builds its input once: the children stay those of the two objects it began with.
  let value = Signal::new(format!("a{O}b{O}c"));
  let objects = Signal::new(vec![swatch(1, 40, 30, false), swatch(5, 20, 20, true)]);
  let controller = TextInputController::new();
  let mut tree = Tree::new();
  tree.resize(800, 600);
  tree.set_root(
    Column::new().size(800.0, 600.0).child(
      TextInput::new(value.clone())
        .id(ID)
        .spans(Signal::new(Vec::new()))
        .objects(objects.clone(), drawn)
        .controller(controller.clone())
        .multiline()
        .width(400.0),
    ),
  );
  let mut app = App::new();
  let count = |tree: &mut Tree, app: &mut App, color: Color| {
    render_pass_with_app(tree, app)
      .rects
      .into_iter()
      .filter(|rect| rect.color == color)
      .count()
  };
  assert_eq!(
    (count(&mut tree, &mut app, RED), count(&mut tree, &mut app, GREEN)),
    (1, 1)
  );
  tree.get_element_by_id_mut(ID).unwrap().focus();
  // The first object is deleted: the child that drew it is not painted, and the second keeps its own.
  controller.select(1, 4).unwrap();
  tree.key_down("Backspace".into(), "Backspace".into(), false, false, false);
  assert_eq!(value.get(), format!("ab{O}c"));
  assert_eq!(objects.get(), [swatch(2, 20, 20, true)]);
  assert_eq!(
    (count(&mut tree, &mut app, RED), count(&mut tree, &mut app, GREEN)),
    (0, 1)
  );
}

#[test]
fn styled_an_object_scrolls_with_the_text_and_is_cut_by_the_input() {
  let lines: Vec<String> = (0..12).map(|line| format!("line {line}")).collect();
  let text = format!("{}\n{O}\nlast", lines.join("\n"));
  let at = text.find(O).unwrap();
  let mut document = Document::new(&text, vec![swatch(at, 60, 50, false)]);
  document.props.rows.set(6);
  document.props.controller.select(0, 0).unwrap();
  document.render();
  let bounds = document.tree.get_element_by_id_mut(ID).unwrap().bounds().unwrap();
  assert!(close(bounds.height, 6.0 * LINE), "{bounds:?}");
  // The box is below what the input shows: nothing of it is painted.
  assert!(document.boxes(RED).is_empty());
  // Scrolled by the wheel, it comes into the input, cut where the input ends.
  let (x, y) = (bounds.x + 100.0, bounds.y + 60.0);
  document.tree.scroll(x, y, 0.0, -8.0 * LINE, ScrollPhase::Scroll);
  let shown = document.the_box(RED);
  assert!(shown.y > bounds.y && shown.y < bounds.y + bounds.height, "{shown:?}");
  assert!(shown.clip.active && shown.clip.y + shown.clip.height <= bounds.y + bounds.height + 0.6);
  assert!(
    shown.y + shown.height > bounds.y + bounds.height,
    "its foot is past the input's: {shown:?}"
  );
  // It moves by what the text moves.
  document.tree.scroll(x, y, 0.0, -LINE, ScrollPhase::Scroll);
  let moved = document.the_box(RED);
  assert!(close(shown.y - moved.y, LINE), "{} {}", shown.y, moved.y);
  // A press on it there selects it.
  pointer_click(&mut document.tree, moved.x + 10.0, moved.y + 10.0, MouseButton::Left);
  assert_eq!(document.selection(), at..at + 3);
}

#[test]
fn styled_a_narrower_input_wraps_the_row_and_the_object_follows() {
  let words = "one two three four five ";
  let mut document = Document::new(&format!("{words}{O}"), vec![swatch(words.len(), 100, 40, false)]);
  let left = document.left();
  let wide = document.the_box(RED);
  assert!(wide.x > left + 100.0, "after the words: {wide:?}");
  document.props.width.set(200.0);
  let narrow = document.the_box(RED);
  assert!(
    close(narrow.x, left) && narrow.y > wide.y,
    "on a row of its own: {narrow:?}"
  );
}

#[test]
fn styled_a_change_reports_the_objects_and_the_app_loads_them_with_the_text() {
  let mut document = Document::new("ab", Vec::new());
  // The app writes the text and its objects: the input shows them.
  document.props.value.set(format!("ab{O}"));
  document.props.objects.set(vec![swatch(2, 40, 30, false)]);
  assert_eq!(document.boxes(RED).len(), 1);
  // The controller puts one at the caret, which is left after it.
  document.props.controller.select(1, 1).unwrap();
  assert!(
    document
      .props
      .controller
      .insert_object(&swatch(0, 20, 20, true))
      .unwrap()
  );
  document.render();
  assert_eq!(document.text(), format!("a{O}b{O}"));
  assert_eq!(document.objects(), [swatch(1, 20, 20, true), swatch(5, 40, 30, false)]);
  assert_eq!(document.caret(), 4);
  assert_eq!((document.boxes(GREEN).len(), document.boxes(RED).len()), (1, 1));
  let rich = document.props.controller.rich_text().unwrap().unwrap();
  assert_eq!(rich.objects, document.objects());
  // One step of the undo takes it back.
  document.shortcut("z", "KeyZ", false);
  assert_eq!(document.text(), format!("ab{O}"));
  assert_eq!(document.objects(), [swatch(2, 40, 30, false)]);
}

#[test]
fn styled_the_handle_of_a_selected_object_makes_it_wider_and_narrower_as_one_undo_step() {
  let mut document = Document::new(&format!("ab{O}cd"), vec![swatch(2, 40, 30, false)]);
  let room = document.the_box(RED);
  // No handle until the object is selected: a press on its corner is a press in the text.
  // The handle is a disc of 12 centred on the lower corner at the end of the box.
  let handle = |document: &mut Document, room: &RectSnapshot| {
    let (x, y) = (room.x + room.width - 6.0, room.y + room.height - 6.0);
    document.render().rects.into_iter().any(|rect| {
      close(rect.width, 12.0) && close(rect.height, 12.0) && close(rect.x, x) && close(rect.y, y) && rect.radii[0] > 5.0
    })
  };
  assert!(!handle(&mut document, &room));
  pointer_click(&mut document.tree, room.x + 20.0, room.y + 10.0, MouseButton::Left);
  assert_eq!(document.selection(), 2..5);
  assert!(
    handle(&mut document, &room),
    "a handle on the corner of the selected object"
  );
  // The handle is dragged 30 to the right: the object is 30 wider as it goes, and stays selected.
  let (x, y) = (room.x + room.width, room.y + room.height);
  document.tree.mouse_move(x, y);
  document.tree.mouse_down(x, y, MouseButton::Left);
  document.tree.mouse_move(x + 12.0, y + 3.0);
  assert_eq!(document.objects()[0].width, Some(52), "it follows the pointer");
  assert!(close(document.the_box(RED).width, 52.0));
  document.tree.mouse_move(x + 30.0, y - 5.0);
  document.tree.mouse_up(x + 30.0, y - 5.0, MouseButton::Left);
  assert_eq!(document.objects(), [swatch(2, 40, 30, false).width(70)]);
  assert!(close(document.the_box(RED).width, 70.0));
  assert_eq!(document.selection(), 2..5);
  assert_eq!(document.text(), format!("ab{O}cd"));
  // Narrower than the least it is not made.
  let room = document.the_box(RED);
  let (x, y) = (room.x + room.width, room.y + room.height);
  document.tree.mouse_move(x, y);
  document.tree.mouse_down(x, y, MouseButton::Left);
  document.tree.mouse_move(x - 500.0, y);
  document.tree.mouse_up(x - 500.0, y, MouseButton::Left);
  assert_eq!(document.objects()[0].width, Some(16));
  // Each drag is one step of the undo.
  document.shortcut("z", "KeyZ", false);
  assert_eq!(document.objects()[0].width, Some(70));
  document.shortcut("z", "KeyZ", false);
  assert_eq!(document.objects()[0].width, Some(40));
  document.shortcut("z", "KeyZ", true);
  assert_eq!(document.objects()[0].width, Some(70));
}

#[test]
fn styled_an_input_whose_objects_are_not_resized_has_no_handle() {
  let value = Signal::new(format!("a{O}b"));
  let objects = Signal::new(vec![swatch(1, 40, 30, false)]);
  let controller = TextInputController::new();
  let mut tree = Tree::new();
  tree.resize(800, 600);
  tree.set_root(
    Column::new().size(800.0, 600.0).child(
      TextInput::new(value)
        .id(ID)
        .spans(Signal::new(Vec::new()))
        .objects(objects.clone(), drawn)
        .controller(controller.clone())
        .multiline()
        .width(400.0),
    ),
  );
  let mut app = App::new();
  let snapshot = render_pass_with_app(&mut tree, &mut app);
  let room = snapshot.rects.iter().find(|rect| rect.color == RED).unwrap().clone();
  pointer_click(&mut tree, room.x + 20.0, room.y + 10.0, MouseButton::Left);
  assert_eq!(controller.selection().unwrap().range(), 1..4);
  let (x, y) = (room.x + room.width - 2.0, room.y + room.height - 2.0);
  tree.mouse_move(x, y);
  tree.mouse_down(x, y, MouseButton::Left);
  tree.mouse_move(x + 40.0, y);
  tree.mouse_up(x + 40.0, y, MouseButton::Left);
  assert_eq!(objects.get(), [swatch(1, 40, 30, false)], "as wide as it was");
}

/// A clipboard of the test's.
#[derive(Default)]
struct Clipboard {
  text: Mutex<Option<String>>,
  markdown: Mutex<Option<String>>,
}

impl ClipboardWriter for Clipboard {
  fn write_text(&self, text: &str) -> bool {
    *self.text.lock().unwrap() = Some(text.to_owned());
    *self.markdown.lock().unwrap() = None;
    true
  }

  fn write_rich(&self, text: &str, markdown: &str) -> bool {
    *self.text.lock().unwrap() = Some(text.to_owned());
    *self.markdown.lock().unwrap() = Some(markdown.to_owned());
    true
  }
}

impl ClipboardReader for Clipboard {
  fn read_text(&self) -> Option<String> {
    self.text.lock().unwrap().clone()
  }

  fn read(&self) -> Option<ClipboardContent> {
    self.read_text().map(ClipboardContent::text)
  }

  fn read_markdown(&self, text: &str) -> Option<String> {
    (self.text.lock().unwrap().as_deref() == Some(text))
      .then(|| self.markdown.lock().unwrap().clone())
      .flatten()
  }
}

#[test]
fn styled_a_copy_of_an_object_is_its_markdown_and_a_paste_of_it_is_the_object_again() {
  let picture = InlineObject::new(4, "30x20").label("red");
  let mut document = Document::new(&format!("see {O} here"), vec![picture.clone()]);
  let clipboard = Arc::new(Clipboard::default());
  document.tree.set_clipboard_writer(Some(clipboard.clone()));
  document.tree.set_clipboard_reader(Some(clipboard.clone()));
  document.props.controller.select(2, 7).unwrap();
  document.shortcut("c", "KeyC", false);
  // The text beside it has the label where the object was; the Markdown has the image.
  assert_eq!(clipboard.text.lock().unwrap().as_deref(), Some("e red"));
  assert_eq!(clipboard.markdown.lock().unwrap().as_deref(), Some("e ![red](30x20)"));
  // Pasted at the end, the object is there again with its record.
  let end = document.text().len();
  document.props.controller.select(end, end).unwrap();
  document.shortcut("v", "KeyV", false);
  assert_eq!(document.text(), format!("see {O} heree {O}"));
  assert_eq!(
    document.objects(),
    [picture.clone(), InlineObject::new(14, "30x20").label("red")]
  );
  assert_eq!(document.boxes(RED).len(), 2);
  // A cut takes the object and its record, and an undo puts both back.
  document.props.controller.select(4, 7).unwrap();
  document.shortcut("x", "KeyX", false);
  assert_eq!(document.text(), "see  heree \u{FFFC}");
  assert_eq!(document.objects(), [InlineObject::new(11, "30x20").label("red")]);
  assert_eq!(clipboard.markdown.lock().unwrap().as_deref(), Some("![red](30x20)"));
  document.shortcut("z", "KeyZ", false);
  assert_eq!(document.objects().len(), 2);
}

#[test]
fn styled_markdown_with_an_image_from_another_app_is_pasted_as_an_object() {
  let mut document = Document::new("ab", Vec::new());
  let clipboard = Arc::new(Clipboard::default());
  document.tree.set_clipboard_reader(Some(clipboard.clone()));
  *clipboard.text.lock().unwrap() = Some("a ![green](20x20) b".to_owned());
  document.shortcut("v", "KeyV", false);
  assert_eq!(document.text(), format!("aba {O} b"));
  assert_eq!(document.objects(), [InlineObject::new(4, "20x20").label("green")]);
  assert_eq!(document.boxes(GREEN).len(), 1);
  let rich: RichText = document.props.controller.rich_text().unwrap().unwrap();
  assert_eq!(rich.to_markdown(), "aba ![green](20x20) b");
}

#[test]
fn styled_what_an_object_draws_takes_neither_the_pointer_nor_the_focus() {
  let pressed = Arc::new(Mutex::new(0));
  let value = Signal::new(format!("a{O}b"));
  let objects = Signal::new(vec![swatch(1, 60, 40, false)]);
  let controller = TextInputController::new();
  let count = pressed.clone();
  let mut tree = Tree::new();
  tree.resize(800, 600);
  tree.set_root(
    Column::new().size(800.0, 600.0).child(
      TextInput::new(value)
        .id(ID)
        .spans(Signal::new(Vec::new()))
        .objects(objects, move |object| {
          let count = count.clone();
          Column::new()
            .child(drawn(object))
            .tab_index(0)
            .on_click(move |_| *count.lock().unwrap() += 1)
            .into()
        })
        .controller(controller.clone())
        .multiline()
        .width(400.0),
    ),
  );
  let mut app = App::new();
  let snapshot = render_pass_with_app(&mut tree, &mut app);
  let room = snapshot.rects.iter().find(|rect| rect.color == RED).unwrap().clone();
  pointer_click(&mut tree, room.x + 20.0, room.y + 20.0, MouseButton::Left);
  assert_eq!(*pressed.lock().unwrap(), 0, "the press is the text's");
  assert_eq!(controller.selection().unwrap().range(), 1..4);
  // Tab from the input does not stop in what the object draws.
  tree.key_down("Tab".into(), "Tab".into(), false, false, false);
  render_pass_with_app(&mut tree, &mut app);
  assert!(tree.get_element_by_id_mut(ID).is_some());
  assert_eq!(*pressed.lock().unwrap(), 0);
}
