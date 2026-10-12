use std::sync::Arc;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Column, Rect, TextInput, TextInputController},
  core::Signal,
  node::Element,
};

use crate::support::pointer_click;

struct EditorState {
  source: Signal<String>,
  controller: TextInputController,
  refocus: Signal<bool>,
}

#[derive(Clone, lurq::DevtoolsInspectable)]
struct EditorProps(#[devtools_ignore] Arc<EditorState>);
impl PartialEq for EditorProps {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct PickerEditor;
impl Component for PickerEditor {
  type Props = EditorProps;
  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let state = ctx.props::<Self::Props>().0.clone();
    let field = TextInput::new(state.source.clone())
      .id("editor")
      .controller(state.controller.clone())
      .width(160.0);
    if state.refocus.get() {
      state.controller.focus(ctx);
    }
    Column::new()
      .child(field)
      .child(Rect::new(160.0, 24.0).id("picker").focusable(true))
  }
}

#[test]
fn input_controller_picker_focus_return_uses_runtime_caret_and_accepts_following_typing() {
  let state = Arc::new(EditorState {
    source: Signal::new("left selected right".to_owned()),
    controller: TextInputController::new(),
    refocus: Signal::new(false),
  });
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<PickerEditor>(&mut app, EditorProps(state.clone()));
  tree.pass_headless(&mut app);
  tree.get_element_by_id_mut("editor").unwrap().focus();
  state.controller.select(5, 13).unwrap();
  let picker = tree.get_element_by_id_mut("picker").unwrap().bounds().unwrap();
  pointer_click(&mut tree, picker.x + 5.0, picker.y + 5.0, MouseButton::Left);
  assert!(!state.controller.element_ref().focused());
  assert!(state.controller.replace_selection("{{task.title}}").unwrap());
  state.refocus.set(true);
  tree.pass_headless(&mut app);
  assert!(state.controller.element_ref().focused());
  tree.key_down("!".into(), "Digit1".into(), false, false, false);
  assert_eq!(state.source.get(), "left {{task.title}}! right");
}

#[test]
fn input_controller_detached_input_refuses_edits() {
  let controller = TextInputController::new();
  let source = Signal::new("unchanged".to_owned());
  let mut tree = Tree::new();
  let mut app = App::new();
  tree.set_root(TextInput::new(source.clone()).controller(controller.clone()));
  tree.pass_headless(&mut app);
  tree.set_root(Rect::new(20.0, 20.0));
  tree.pass_headless(&mut app);
  assert_eq!(
    controller.replace_selection("X"),
    Err(lurq::components::InputEditError::NotMounted)
  );
  assert_eq!(source.get(), "unchanged");
}
