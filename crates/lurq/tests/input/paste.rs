//! A paste into a text input delivers one `PasteEvent` to the input and its
//! ancestors before the text is inserted, and a synthetic Ctrl+V never
//! reads the OS clipboard.

use std::{
  path::PathBuf,
  sync::{Arc, Mutex},
};

use lurq::{
  app::{
    Tree,
    events::{ClipboardContent, ClipboardImage, MouseButton, PasteEvent, is_synthetic_input},
    synthetic_input::{self, SyntheticInput, SyntheticModifiers},
  },
  components::{Column, TextInput},
  core::Signal,
};

use crate::support::{pointer_click, run_pass};

fn focused_input(value: &Signal<String>) -> Tree {
  let mut runtime = Tree::new();
  runtime.set_root(TextInput::new(value.clone()).id("input"));
  focus(runtime)
}

fn focus(mut runtime: Tree) -> Tree {
  run_pass(&mut runtime);
  let (x, y) = runtime
    .get_element_by_id_mut("input")
    .expect("input")
    .bounds()
    .expect("laid out")
    .center();
  pointer_click(&mut runtime, x, y, MouseButton::Left);
  runtime
}

#[test]
fn a_paste_inserts_the_text_at_the_caret() {
  let value = Signal::new("ab".to_owned());
  let mut runtime = focused_input(&value);
  runtime.key_down("End".into(), "End".into(), false, false, false);

  assert!(runtime.paste(ClipboardContent::text("cd")));

  assert_eq!(value.get(), "abcd");
}

#[test]
fn the_paste_event_runs_before_the_insertion() {
  let value = Signal::new("old".to_owned());
  let seen = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = Tree::new();
  let log = seen.clone();
  let observed = value.clone();
  runtime.set_root(
    TextInput::new(value.clone())
      .id("input")
      .on_paste(move |event: PasteEvent| {
        log
          .lock()
          .unwrap()
          .push((observed.get(), event.text().map(str::to_owned)));
      }),
  );
  let mut runtime = focus(runtime);
  runtime.key_down("End".into(), "End".into(), false, false, false);

  runtime.paste(ClipboardContent::text("new"));

  assert_eq!(
    seen.lock().unwrap().as_slice(),
    &[("old".to_owned(), Some("new".to_owned()))],
    "the handler sees the value before the paste"
  );
  assert_eq!(value.get(), "oldnew");
}

#[test]
fn prevent_default_keeps_the_text_out_of_the_input() {
  let value = Signal::new(String::new());
  let mut runtime = Tree::new();
  runtime.set_root(
    TextInput::new(value.clone())
      .id("input")
      .on_paste(|event: PasteEvent| event.prevent_default()),
  );
  let mut runtime = focus(runtime);

  assert!(
    runtime.paste(ClipboardContent::text("kept out")),
    "a prevented paste is handled"
  );

  assert_eq!(value.get(), "");
}

#[test]
fn an_image_and_files_reach_the_handler_and_insert_nothing() {
  let value = Signal::new(String::new());
  let seen = Arc::new(Mutex::new(None));
  let mut runtime = Tree::new();
  let log = seen.clone();
  runtime.set_root(
    TextInput::new(value.clone())
      .id("input")
      .on_paste(move |event: PasteEvent| {
        let image = event.image().map(|image| (image.width(), image.height()));
        *log.lock().unwrap() = Some((event.kinds(), image, event.files().to_vec()));
        event.prevent_default();
      }),
  );
  let mut runtime = focus(runtime);
  let image = ClipboardImage::from_rgba(2, 1, vec![255; 8]).expect("2x1 RGBA");
  let content = ClipboardContent::image(image).with_files([PathBuf::from("C:/shots/a.png")]);

  runtime.paste(content);

  let (kinds, image, files) = seen.lock().unwrap().clone().expect("the handler ran");
  assert!(kinds.image && kinds.files && !kinds.text);
  assert_eq!(image, Some((2, 1)));
  assert_eq!(files, vec![PathBuf::from("C:/shots/a.png")]);
  assert_eq!(value.get(), "");
}

#[test]
fn a_paste_without_text_or_handler_is_not_handled() {
  let value = Signal::new(String::new());
  let mut runtime = focused_input(&value);

  assert!(!runtime.paste(ClipboardContent::files([PathBuf::from("a.txt")])));
  assert!(!runtime.paste(ClipboardContent::text("")));
  assert_eq!(value.get(), "");
}

#[test]
fn the_paste_event_bubbles_from_the_input_to_its_ancestors() {
  let value = Signal::new(String::new());
  let order = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = Tree::new();
  let (outer, inner) = (order.clone(), order.clone());
  runtime.set_root(
    Column::new()
      .on_paste(move |event: PasteEvent| {
        outer
          .lock()
          .unwrap()
          .push(("column", event.target_id == event.input_id));
      })
      .child(
        TextInput::new(value.clone())
          .id("input")
          .on_paste(move |event: PasteEvent| {
            inner.lock().unwrap().push(("input", event.target_id == event.input_id));
          }),
      ),
  );
  let mut runtime = focus(runtime);

  runtime.paste(ClipboardContent::text("x"));

  assert_eq!(order.lock().unwrap().as_slice(), &[("input", true), ("column", false)]);
  assert_eq!(value.get(), "x");
}

#[test]
fn stop_propagation_keeps_the_paste_from_the_ancestors() {
  let value = Signal::new(String::new());
  let column_saw = Arc::new(Mutex::new(false));
  let mut runtime = Tree::new();
  let flag = column_saw.clone();
  runtime.set_root(
    Column::new()
      .on_paste(move |_: PasteEvent| *flag.lock().unwrap() = true)
      .child(
        TextInput::new(value.clone())
          .id("input")
          .on_paste(|event: PasteEvent| event.stop_propagation()),
      ),
  );
  let mut runtime = focus(runtime);

  runtime.paste(ClipboardContent::text("x"));

  assert!(!*column_saw.lock().unwrap());
  assert_eq!(value.get(), "x", "stopping the propagation keeps the default");
}

#[test]
fn a_paste_without_a_focused_input_does_nothing() {
  let value = Signal::new(String::new());
  let mut runtime = Tree::new();
  runtime.set_root(TextInput::new(value.clone()).id("input"));
  run_pass(&mut runtime);

  assert!(!runtime.paste(ClipboardContent::text("nowhere")));
  assert_eq!(value.get(), "");
}

#[test]
fn a_synthetic_paste_carries_its_content_and_is_marked_synthetic() {
  let value = Signal::new(String::new());
  let synthetic = Arc::new(Mutex::new(None));
  let mut runtime = Tree::new();
  let flag = synthetic.clone();
  runtime.set_root(
    TextInput::new(value.clone())
      .id("input")
      .on_paste(move |_: PasteEvent| {
        *flag.lock().unwrap() = Some(is_synthetic_input());
      }),
  );
  let mut runtime = focus(runtime);

  synthetic_input::apply(&mut runtime, &SyntheticInput::paste(ClipboardContent::text("agent")));

  assert_eq!(*synthetic.lock().unwrap(), Some(true));
  assert_eq!(value.get(), "agent");
}

/// The person's clipboard is theirs: a synthetic Ctrl+V (an MCP client's
/// `key` action) is refused instead of reading it, whatever it holds.
#[test]
fn a_synthetic_ctrl_v_does_not_read_the_os_clipboard() {
  let value = Signal::new(String::new());
  let seen = Arc::new(Mutex::new(0));
  let mut runtime = Tree::new();
  let count = seen.clone();
  runtime.set_root(
    TextInput::new(value.clone())
      .id("input")
      .on_paste(move |_: PasteEvent| {
        *count.lock().unwrap() += 1;
      }),
  );
  let mut runtime = focus(runtime);
  let ctrl = SyntheticModifiers::default().ctrl();
  let meta = SyntheticModifiers::default().meta();
  let shift = SyntheticModifiers::default().shift();

  for input in [
    SyntheticInput::key_down("v").with_modifiers(ctrl),
    SyntheticInput::key_down("v").with_modifiers(meta),
    SyntheticInput::key_down("Insert").with_modifiers(shift),
  ] {
    synthetic_input::apply(&mut runtime, &input);
  }

  assert_eq!(*seen.lock().unwrap(), 0, "no paste event without content");
  assert_eq!(value.get(), "");
}
