//! The commands of editing: which keys they are, in any layout, and how one is run on the focus, by its key or by
//! `Ctx::edit`: a focused text input first, then the focused element's handlers and its ancestors', innermost first.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App, ClipboardReader, ClipboardWriter, Tree,
    component::Component,
    ctx::Ctx,
    events::{ClipboardContent, CopyEvent, EditCommand, EditEvent, KeyboardEvent, MouseButton, PasteEvent},
  },
  components::{Column, Rect, Row, TextInput},
  core::{NodeId, Signal},
  node::Element,
};

use crate::support::{pointer_click, run_pass};

const APPLE: bool = cfg!(any(target_os = "macos", target_os = "ios"));

/// A key with the platform's shortcut key held: Cmd on Apple's platforms, Ctrl elsewhere.
fn shortcut(key: &str, code: &str, shift: bool) -> KeyboardEvent {
  KeyboardEvent::new(key, code, shift, !APPLE, false, APPLE, NodeId::UNASSIGNED)
}

fn plain(key: &str, code: &str, shift: bool, ctrl: bool) -> KeyboardEvent {
  KeyboardEvent::new(key, code, shift, ctrl, false, false, NodeId::UNASSIGNED)
}

/// Presses a key with the platform's shortcut key held, and lets it go.
fn press(runtime: &mut Tree, key: &str, code: &str, shift: bool) {
  runtime.key_down_with_meta(key.into(), code.into(), shift, !APPLE, false, APPLE);
  runtime.key_up_with_meta(key.into(), code.into(), shift, !APPLE, false, APPLE);
}

#[test]
fn undo_and_redo_are_z_and_y_by_the_letter_or_by_its_place() {
  assert_eq!(shortcut("z", "KeyZ", false).edit_command(), Some(EditCommand::Undo));
  assert_eq!(shortcut("Z", "KeyZ", true).edit_command(), Some(EditCommand::Redo));
  // A Ukrainian layout: the key says "я", its place is Z's.
  assert_eq!(shortcut("я", "KeyZ", false).edit_command(), Some(EditCommand::Undo));
  assert_eq!(shortcut("Я", "KeyZ", true).edit_command(), Some(EditCommand::Redo));
  assert_eq!(
    shortcut("y", "KeyY", false).edit_command(),
    (!APPLE).then_some(EditCommand::Redo),
    "Y redoes, but on a Mac"
  );
  assert_eq!(shortcut("Y", "KeyY", true).edit_command(), None, "Shift+Y is nothing");
  // A German keyboard says Z in Y's place: the letter it says wins over the place.
  assert_eq!(shortcut("z", "KeyY", false).edit_command(), Some(EditCommand::Undo));
}

#[test]
fn the_shortcut_key_is_the_platform_s_and_alt_makes_it_no_command() {
  let with =
    |ctrl: bool, alt: bool, meta: bool| KeyboardEvent::new("z", "KeyZ", false, ctrl, alt, meta, NodeId::UNASSIGNED);
  assert_eq!(with(!APPLE, false, APPLE).edit_command(), Some(EditCommand::Undo));
  assert_eq!(
    with(APPLE, false, !APPLE).edit_command(),
    None,
    "the other platform's key"
  );
  assert_eq!(
    with(!APPLE, true, APPLE).edit_command(),
    None,
    "Alt held: AltGr is Ctrl+Alt"
  );
  assert_eq!(with(false, false, false).edit_command(), None, "Z alone is a letter");
  assert_eq!(
    EditCommand::Undo.accelerator().to_string(),
    if APPLE { "Meta+Z" } else { "Ctrl+Z" }
  );
  assert_eq!(
    EditCommand::Redo.accelerator().to_string(),
    if APPLE { "Shift+Meta+Z" } else { "Ctrl+Y" }
  );
}

#[test]
fn cut_copy_paste_and_select_all_in_any_layout_and_the_old_keys() {
  assert_eq!(shortcut("c", "KeyC", false).edit_command(), Some(EditCommand::Copy));
  assert_eq!(shortcut("x", "KeyX", false).edit_command(), Some(EditCommand::Cut));
  assert_eq!(shortcut("v", "KeyV", false).edit_command(), Some(EditCommand::Paste));
  assert_eq!(
    shortcut("a", "KeyA", false).edit_command(),
    Some(EditCommand::SelectAll)
  );
  assert_eq!(
    shortcut("A", "KeyA", true).edit_command(),
    None,
    "Shift+A selects nothing"
  );
  // Cyrillic: С, Ч, М and Ф are where C, X, V and A are.
  assert_eq!(shortcut("с", "KeyC", false).edit_command(), Some(EditCommand::Copy));
  assert_eq!(shortcut("ч", "KeyX", false).edit_command(), Some(EditCommand::Cut));
  assert_eq!(shortcut("м", "KeyV", false).edit_command(), Some(EditCommand::Paste));
  assert_eq!(
    shortcut("ф", "KeyA", false).edit_command(),
    Some(EditCommand::SelectAll)
  );
  // The keys of the old IBM keyboard.
  assert_eq!(
    plain("Insert", "Insert", false, true).edit_command(),
    Some(EditCommand::Copy)
  );
  assert_eq!(
    plain("Insert", "Insert", true, false).edit_command(),
    Some(EditCommand::Paste)
  );
  assert_eq!(
    plain("Delete", "Delete", true, false).edit_command(),
    Some(EditCommand::Cut)
  );
  assert_eq!(plain("Delete", "Delete", false, false).edit_command(), None);
  assert_eq!(plain("c", "KeyC", false, false).edit_command(), None);
}

/// Who heard what, in order.
type Log = Arc<Mutex<Vec<String>>>;

fn log() -> Log {
  Arc::new(Mutex::new(Vec::new()))
}

fn heard(log: &Log) -> Vec<String> {
  log.lock().unwrap().clone()
}

/// A handler that writes `name` and the command, and passes it on where `passes`.
fn handler(log: &Log, name: &'static str, passes: bool) -> impl Fn(EditEvent) + Send + Sync + 'static {
  let log = log.clone();
  move |event: EditEvent| {
    log.lock().unwrap().push(format!("{name} {:?}", event.command()));
    if passes {
      event.pass_on();
    }
  }
}

fn focus(runtime: &mut Tree, id: &str) {
  run_pass(runtime);
  let (x, y) = runtime
    .get_element_by_id_mut(id)
    .expect("the element")
    .bounds()
    .expect("laid out")
    .center();
  pointer_click(runtime, x, y, MouseButton::Left);
}

/// A focusable box in `inner`, in `outer`, each with its own handlers.
fn nested(log: &Log, inner_passes: bool) -> Tree {
  let mut runtime = Tree::new();
  runtime.set_root(
    Column::new()
      .on_undo(handler(log, "outer", false))
      .on_redo(handler(log, "outer", false))
      .child(
        Row::new()
          .on_undo(handler(log, "inner", inner_passes))
          .on_select_all(handler(log, "inner", inner_passes))
          .child(Row::new().size(80.0, 30.0).tab_index(0).id("box")),
      ),
  );
  focus(&mut runtime, "box");
  runtime
}

#[test]
fn a_command_goes_to_the_innermost_element_that_hears_it() {
  let log = log();
  let mut runtime = nested(&log, false);
  press(&mut runtime, "z", "KeyZ", false);
  assert_eq!(heard(&log), ["inner Undo"], "the inner took it: the outer is not told");
  press(&mut runtime, "Z", "KeyZ", true);
  assert_eq!(heard(&log), ["inner Undo", "outer Redo"], "the inner hears no redo");
  press(&mut runtime, "ф", "KeyA", false);
  assert_eq!(heard(&log).last().map(String::as_str), Some("inner SelectAll"));
}

#[test]
fn a_handler_that_did_nothing_passes_the_command_out() {
  let log = log();
  let mut runtime = nested(&log, true);
  press(&mut runtime, "я", "KeyZ", false);
  assert_eq!(heard(&log), ["inner Undo", "outer Undo"]);
}

#[test]
fn a_key_the_app_binds_itself_is_no_command() {
  let log = log();
  let mut runtime = Tree::new();
  runtime.set_root(
    Column::new()
      .on_key_down(|event: KeyboardEvent| {
        if event.edit_command() == Some(EditCommand::Undo) {
          event.prevent_default();
        }
      })
      .on_undo(handler(&log, "outer", false))
      .child(Row::new().size(80.0, 30.0).tab_index(0).id("box")),
  );
  focus(&mut runtime, "box");
  press(&mut runtime, "z", "KeyZ", false);
  assert!(heard(&log).is_empty(), "the app's own Ctrl+Z won");
}

#[test]
fn nothing_focused_hears_nothing() {
  let log = log();
  let mut runtime = Tree::new();
  runtime.set_root(
    Column::new()
      .on_undo(handler(&log, "outer", false))
      .child(Row::new().size(80.0, 30.0)),
  );
  run_pass(&mut runtime);
  press(&mut runtime, "z", "KeyZ", false);
  assert!(heard(&log).is_empty());
}

#[test]
fn a_text_input_takes_back_its_own_edits_before_the_elements_around_it() {
  let log = log();
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();
  runtime.set_root(
    Column::new()
      .on_undo(handler(&log, "form", false))
      .child(TextInput::new(value.clone()).id("input")),
  );
  focus(&mut runtime, "input");
  runtime.key_down("B".into(), "KeyB".into(), false, false, false);
  assert_eq!(value.get(), "AB");
  press(&mut runtime, "я", "KeyZ", false);
  assert_eq!(value.get(), "A", "the input's own undo, in a Ukrainian layout");
  assert!(heard(&log).is_empty(), "the form is not told");
  press(&mut runtime, "Я", "KeyZ", true);
  assert_eq!(value.get(), "AB", "Ctrl+Shift+Z by its place");
  press(&mut runtime, "z", "KeyZ", false);
  press(&mut runtime, "z", "KeyZ", false);
  assert_eq!(value.get(), "A");
  assert_eq!(
    heard(&log),
    ["form Undo"],
    "with nothing left to take back, the form's history"
  );
}

/// A clipboard of the test's own: what copies wrote, and what a paste reads.
#[derive(Default)]
struct Clipboard(Mutex<Vec<String>>);

impl ClipboardWriter for Clipboard {
  fn write_text(&self, text: &str) -> bool {
    self.0.lock().unwrap().push(text.to_owned());
    true
  }
}

impl ClipboardReader for Clipboard {
  fn read_text(&self) -> Option<String> {
    Some("pasted".to_owned())
  }

  fn read(&self) -> Option<ClipboardContent> {
    self.read_text().map(ClipboardContent::text)
  }
}

impl Clipboard {
  fn install(runtime: &mut Tree) -> Arc<Self> {
    let clipboard = Arc::new(Self::default());
    runtime.set_clipboard_writer(Some(clipboard.clone()));
    runtime.set_clipboard_reader(Some(clipboard.clone()));
    clipboard
  }

  fn written(&self) -> Vec<String> {
    self.0.lock().unwrap().clone()
  }
}

#[test]
fn a_text_input_selects_copies_cuts_and_pastes_in_a_cyrillic_layout() {
  let value = Signal::new("word".to_owned());
  let mut runtime = Tree::new();
  runtime.set_root(TextInput::new(value.clone()).id("input"));
  focus(&mut runtime, "input");
  let clipboard = Clipboard::install(&mut runtime);
  press(&mut runtime, "ф", "KeyA", false);
  press(&mut runtime, "с", "KeyC", false);
  assert_eq!(clipboard.written(), ["word"], "Ctrl+A then Ctrl+C");
  press(&mut runtime, "ч", "KeyX", false);
  assert_eq!(value.get(), "");
  assert_eq!(clipboard.written(), ["word", "word"]);
  press(&mut runtime, "м", "KeyV", false);
  assert_eq!(value.get(), "pasted");
}

#[test]
fn the_old_keys_copy_paste_and_cut_in_a_text_input() {
  let value = Signal::new("ab".to_owned());
  let mut runtime = Tree::new();
  runtime.set_root(TextInput::new(value.clone()).id("input"));
  focus(&mut runtime, "input");
  let clipboard = Clipboard::install(&mut runtime);
  press(&mut runtime, "a", "KeyA", false);
  runtime.key_down("Insert".into(), "Insert".into(), false, true, false);
  assert_eq!(clipboard.written(), ["ab"], "Ctrl+Insert copies");
  runtime.key_down("Delete".into(), "Delete".into(), true, false, false);
  assert_eq!(
    (value.get().as_str(), clipboard.written().len()),
    ("", 2),
    "Shift+Delete cuts"
  );
  runtime.key_down("Insert".into(), "Insert".into(), true, false, false);
  assert_eq!(value.get(), "pasted", "Shift+Insert pastes");
}

#[test]
fn a_copy_handler_that_sets_no_text_writes_nothing_and_a_cyrillic_copy_reaches_one_that_does() {
  let log = log();
  let mut runtime = Tree::new();
  let (outer, inner, pastes) = (log.clone(), log.clone(), log.clone());
  runtime.set_root(
    Column::new()
      .on_copy(move |event: CopyEvent| {
        outer.lock().unwrap().push(format!("outer cut={}", event.is_cut()));
        event.set_text("cells");
      })
      .on_paste(move |event: PasteEvent| {
        pastes
          .lock()
          .unwrap()
          .push(format!("paste {}", event.text().unwrap_or_default()));
      })
      .child(
        Row::new()
          .on_copy(move |_: CopyEvent| inner.lock().unwrap().push("inner".to_owned()))
          .child(Row::new().size(80.0, 30.0).tab_index(0).id("cell")),
      ),
  );
  focus(&mut runtime, "cell");
  let clipboard = Clipboard::install(&mut runtime);
  press(&mut runtime, "с", "KeyC", false);
  press(&mut runtime, "ч", "KeyX", false);
  press(&mut runtime, "м", "KeyV", false);
  assert_eq!(
    heard(&log),
    ["inner", "outer cut=false", "inner", "outer cut=true", "paste pasted"]
  );
  assert_eq!(clipboard.written(), ["cells", "cells"]);
}

/// An app with an Edit menu of its own: a text input, a box that keeps a history, and tools that run the commands
/// without a key and take no focus.
struct EditMenu {
  value: Signal<String>,
  log: Log,
}

thread_local! {
  /// What the test gives its app: the input's value, and the log of the box.
  static GIVEN: std::cell::RefCell<Option<(Signal<String>, Log)>> = const { std::cell::RefCell::new(None) };
}

impl Component for EditMenu {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    let (value, log) = GIVEN.with(|given| given.borrow().clone()).expect("given");
    Self { value, log }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let tool = |id: &'static str, command: EditCommand| {
      let edit = ctx.edit_runner();
      Rect::new(40.0, 20.0)
        .id(id)
        .focusable(false)
        .on_click(move |_: lurq::app::events::MouseEvent| edit.run(command))
    };
    Column::new()
      .child(
        Row::new()
          .child(tool("undo", EditCommand::Undo))
          .child(tool("copy", EditCommand::Copy))
          .child(tool("select-all", EditCommand::SelectAll)),
      )
      .child(TextInput::new(self.value.clone()).id("input"))
      .child(
        Row::new()
          .on_undo(handler(&self.log, "box", false))
          .child(Row::new().size(80.0, 30.0).tab_index(0).id("box")),
      )
  }
}

#[test]
fn a_tool_runs_a_command_on_what_has_the_focus_through_ctx() {
  let (value, log) = (Signal::new("A".to_owned()), log());
  let mut app = App::new();
  let mut runtime = Tree::new();
  GIVEN.with(|given| *given.borrow_mut() = Some((value.clone(), log.clone())));
  runtime.mount_root::<EditMenu>(&mut app, ());
  runtime.pass_headless(&mut app);
  let clipboard = Clipboard::install(&mut runtime);
  let click = |runtime: &mut Tree, app: &mut App, id: &str| {
    let (x, y) = runtime
      .get_element_by_id_mut(id)
      .expect("the element")
      .bounds()
      .expect("laid out")
      .center();
    pointer_click(runtime, x, y, MouseButton::Left);
    runtime.pass_headless(app);
  };
  click(&mut runtime, &mut app, "input");
  runtime.key_down("B".into(), "KeyB".into(), false, false, false);
  assert_eq!(value.get(), "AB");
  click(&mut runtime, &mut app, "undo");
  assert_eq!(value.get(), "A", "Undo of the menu took back the input's edit");
  click(&mut runtime, &mut app, "select-all");
  click(&mut runtime, &mut app, "copy");
  assert_eq!(clipboard.written(), ["A"], "Select All and Copy of the menu");
  click(&mut runtime, &mut app, "box");
  click(&mut runtime, &mut app, "undo");
  assert_eq!(heard(&log), ["box Undo"], "the box focused hears the menu's Undo");
}

/// A text input whose app reads what is typed, and so builds it again after each key, as a form does.
struct Typed {
  value: Signal<String>,
}

thread_local! {
  static TYPED: std::cell::RefCell<Option<Signal<String>>> = const { std::cell::RefCell::new(None) };
}

impl Component for Typed {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      value: TYPED.with(|given| given.borrow().clone()).expect("given"),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let length = self.value.get().len();
    Column::new()
      .child(TextInput::new(self.value.clone()).id("input"))
      .child(Rect::new(length as f32 + 1.0, 4.0))
  }
}

#[test]
fn undo_takes_back_what_was_typed_as_the_platform_groups_it_in_an_input_built_again_after_each_key() {
  let value = Signal::new(String::new());
  let mut app = App::new();
  let mut runtime = Tree::new();
  TYPED.with(|given| *given.borrow_mut() = Some(value.clone()));
  runtime.mount_root::<Typed>(&mut app, ());
  runtime.pass_headless(&mut app);
  focus(&mut runtime, "input");
  for character in "hello world".chars() {
    let key = character.to_string();
    runtime.key_down_with_meta(key.clone(), String::new(), false, false, false, false);
    runtime.key_up_with_meta(key, String::new(), false, false, false, false);
    runtime.pass_headless(&mut app);
  }
  assert_eq!(value.get(), "hello world");
  press(&mut runtime, "z", "KeyZ", false);
  runtime.pass_headless(&mut app);
  // A run of typing on Apple's systems and elsewhere; a word with the spaces after it on Windows.
  assert_eq!(value.get(), if cfg!(windows) { "hello " } else { "" });
}
