use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::{MouseButton, MouseEventKind},
  },
  components::{Column, Rect, Row, Stack},
  core::{ElementRef, Signal},
  node::{Element, HitTestBehavior, color::Color},
};

use crate::support::run_pass;

#[derive(Clone, lurq::DevtoolsInspectable)]
struct Shared<T>(std::sync::Arc<T>);

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    std::sync::Arc::ptr_eq(&self.0, &other.0)
  }
}

impl<T> std::fmt::Debug for Shared<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Shared")
      .field(&(std::sync::Arc::as_ptr(&self.0) as usize))
      .finish()
  }
}

#[derive(Default)]
struct OutsideClickState {
  enabled: Option<Signal<bool>>,
  outside_clicks: u32,
}

struct OutsideClickRoot {
  enabled: Signal<bool>,
  panel_ref: ElementRef,
  state: std::sync::Arc<std::sync::Mutex<OutsideClickState>>,
}

#[derive(Clone, Copy, Debug)]
struct OutsideClickEventSnapshot {
  x: f32,
  y: f32,
  button: MouseButton,
  is_click: bool,
  target_assigned: bool,
}

#[derive(Default)]
struct MultiOutsideClickState {
  first_clicks: u32,
  second_clicks: u32,
  last_event: Option<OutsideClickEventSnapshot>,
}

struct MultiOutsideClickRoot {
  first_ref: ElementRef,
  second_ref: ElementRef,
  state: std::sync::Arc<std::sync::Mutex<MultiOutsideClickState>>,
}

impl Component for OutsideClickRoot {
  type Props = Shared<std::sync::Mutex<OutsideClickState>>;

  fn create(ctx: &mut Ctx) -> Self {
    let enabled = ctx.signal(true);
    let state = ctx.props::<Self::Props>().0.clone();
    state.lock().unwrap().enabled = Some(enabled.clone());
    Self {
      enabled,
      panel_ref: ElementRef::new(),
      state,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    if self.enabled.get() {
      let state = self.state.clone();
      ctx.on_click_outside(self.panel_ref.clone(), move |_| {
        state.lock().unwrap().outside_clicks += 1;
      });
    }

    Column::new()
      .child(
        Rect::new(100.0, 40.0)
          .background("#22c55e")
          .ref_element(self.panel_ref.clone()),
      )
      .child(Rect::new(100.0, 40.0).background("#ef4444"))
  }
}

impl Component for MultiOutsideClickRoot {
  type Props = Shared<std::sync::Mutex<MultiOutsideClickState>>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      first_ref: ElementRef::new(),
      second_ref: ElementRef::new(),
      state: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let state = self.state.clone();
    ctx.on_click_outside(self.first_ref.clone(), move |event| {
      let mut state = state.lock().unwrap();
      state.first_clicks += 1;
      state.last_event = Some(OutsideClickEventSnapshot {
        x: event.x,
        y: event.y,
        button: event.button,
        is_click: matches!(event.kind, MouseEventKind::Click),
        target_assigned: event.target_id.is_assigned(),
      });
    });

    let state = self.state.clone();
    ctx.on_click_outside(self.second_ref.clone(), move |event| {
      let mut state = state.lock().unwrap();
      state.second_clicks += 1;
      state.last_event = Some(OutsideClickEventSnapshot {
        x: event.x,
        y: event.y,
        button: event.button,
        is_click: matches!(event.kind, MouseEventKind::Click),
        target_assigned: event.target_id.is_assigned(),
      });
    });

    Column::new()
      .child(
        Row::new()
          .child(
            Rect::new(100.0, 40.0)
              .background("#22c55e")
              .ref_element(self.first_ref.clone()),
          )
          .child(
            Rect::new(100.0, 40.0)
              .background("#38bdf8")
              .ref_element(self.second_ref.clone()),
          ),
      )
      .child(Rect::new(200.0, 40.0).background("#ef4444"))
  }
}

#[test]
fn release_over_click_target_does_not_click_when_press_started_elsewhere() {
  let clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Rect::new(100.0, 40.0)
      .background("#22c55e")
      .on_click({
        let clicks = clicks.clone();
        move |_| clicks.update(|count| *count += 1)
      }),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.mouse_down(x + rect.width + 20.0, y, MouseButton::Left);
  runtime.mouse_up(x, y, MouseButton::Left);

  assert_eq!(clicks.get(), 0);
}

#[test]
fn release_far_from_press_clicks_same_target() {
  let clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Rect::new(100.0, 40.0)
      .background("#22c55e")
      .on_click({
        let clicks = clicks.clone();
        move |_| clicks.update(|count| *count += 1)
      }),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;

  runtime.mouse_down(rect.x + 10.0, y, MouseButton::Left);
  runtime.mouse_up(rect.x + 90.0, y, MouseButton::Left);

  assert_eq!(clicks.get(), 1);
}

#[test]
fn release_near_press_clicks_same_target() {
  let clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Rect::new(100.0, 40.0)
      .background("#22c55e")
      .on_click({
        let clicks = clicks.clone();
        move |_| clicks.update(|count| *count += 1)
      }),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.mouse_down(x, y, MouseButton::Left);
  runtime.mouse_up(x + 2.0, y, MouseButton::Left);

  assert_eq!(clicks.get(), 1);
}

#[test]
fn non_left_buttons_do_not_fire_on_click() {
  let clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(Rect::new(100.0, 40.0).background("#22c55e").on_click({
    let clicks = clicks.clone();
    move |_| clicks.update(|count| *count += 1)
  }));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  for button in [MouseButton::Right, MouseButton::Middle, MouseButton::Other(4)] {
    runtime.mouse_down(x, y, button);
    runtime.mouse_up(x, y, button);
  }

  assert_eq!(clicks.get(), 0);
}

#[test]
fn on_mouse_click_fires_only_for_matching_button() {
  let clicks = Signal::new(0);
  let left_mouse_clicks = Signal::new(0);
  let right_mouse_clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    Rect::new(100.0, 40.0)
      .background("#22c55e")
      .on_click({
        let clicks = clicks.clone();
        move |_| clicks.update(|count| *count += 1)
      })
      .on_mouse_click(MouseButton::Left, {
        let left_mouse_clicks = left_mouse_clicks.clone();
        move |_| left_mouse_clicks.update(|count| *count += 1)
      })
      .on_mouse_click(MouseButton::Right, {
        let right_mouse_clicks = right_mouse_clicks.clone();
        move |_| right_mouse_clicks.update(|count| *count += 1)
      }),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.mouse_down(x, y, MouseButton::Left);
  runtime.mouse_up(x, y, MouseButton::Left);

  assert_eq!(clicks.get(), 1);
  assert_eq!(left_mouse_clicks.get(), 1);
  assert_eq!(right_mouse_clicks.get(), 0);

  runtime.mouse_down(x, y, MouseButton::Right);
  runtime.mouse_up(x, y, MouseButton::Right);

  assert_eq!(clicks.get(), 1);
  assert_eq!(left_mouse_clicks.get(), 1);
  assert_eq!(right_mouse_clicks.get(), 1);

  runtime.mouse_down(x, y, MouseButton::Middle);
  runtime.mouse_up(x, y, MouseButton::Middle);

  assert_eq!(clicks.get(), 1);
  assert_eq!(left_mouse_clicks.get(), 1);
  assert_eq!(right_mouse_clicks.get(), 1);
}

#[test]
fn ctx_on_click_outside_fires_for_left_clicks_outside_ref() {
  let state = std::sync::Arc::new(std::sync::Mutex::new(OutsideClickState::default()));
  let mut app = App::new();
  let mut runtime = Tree::new();
  runtime.mount_root::<OutsideClickRoot>(&mut app, Shared(state.clone()));
  run_pass(&mut runtime);

  let panel = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .bounds();
  let outside = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();

  let (inside_x, inside_y) = panel.center();
  runtime.mouse_down(inside_x, inside_y, MouseButton::Left);
  runtime.mouse_up(inside_x, inside_y, MouseButton::Left);
  assert_eq!(state.lock().unwrap().outside_clicks, 0);

  let (outside_x, outside_y) = outside.center();
  runtime.mouse_down(outside_x, outside_y, MouseButton::Right);
  runtime.mouse_up(outside_x, outside_y, MouseButton::Right);
  assert_eq!(state.lock().unwrap().outside_clicks, 0);

  runtime.mouse_down(outside_x, outside_y, MouseButton::Left);
  runtime.mouse_up(outside_x, outside_y, MouseButton::Left);
  assert_eq!(state.lock().unwrap().outside_clicks, 1);

  state.lock().unwrap().enabled.as_ref().unwrap().set(false);
  run_pass(&mut runtime);

  runtime.mouse_down(outside_x, outside_y, MouseButton::Left);
  runtime.mouse_up(outside_x, outside_y, MouseButton::Left);
  assert_eq!(state.lock().unwrap().outside_clicks, 1);
}

#[test]
fn ctx_on_click_outside_handles_multiple_hooks_and_event_fields() {
  let state = std::sync::Arc::new(std::sync::Mutex::new(MultiOutsideClickState::default()));
  let mut app = App::new();
  let mut runtime = Tree::new();
  runtime.mount_root::<MultiOutsideClickRoot>(&mut app, Shared(state.clone()));
  run_pass(&mut runtime);

  let first = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .bounds();
  let outside = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();

  let (first_x, first_y) = first.center();
  runtime.mouse_down(first_x, first_y, MouseButton::Left);
  runtime.mouse_up(first_x, first_y, MouseButton::Left);
  {
    let state = state.lock().unwrap();
    assert_eq!(state.first_clicks, 0);
    assert_eq!(state.second_clicks, 1);
    let event = state.last_event.expect("second hook should receive click event");
    assert_eq!(event.x, first_x);
    assert_eq!(event.y, first_y);
    assert_eq!(event.button, MouseButton::Left);
    assert!(event.is_click);
    assert!(event.target_assigned);
  }

  let (outside_x, outside_y) = outside.center();
  runtime.mouse_down(outside_x, outside_y, MouseButton::Left);
  runtime.mouse_up(outside_x, outside_y, MouseButton::Left);
  {
    let state = state.lock().unwrap();
    assert_eq!(state.first_clicks, 1);
    assert_eq!(state.second_clicks, 2);
    let event = state.last_event.expect("outside click should receive click event");
    assert_eq!(event.x, outside_x);
    assert_eq!(event.y, outside_y);
    assert_eq!(event.button, MouseButton::Left);
    assert!(event.is_click);
    assert!(event.target_assigned);
  }
}

#[test]
fn child_press_parent_release_clicks_parent() {
  let parent_clicks = Signal::new(0);
  let child_clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    Column::new()
      .size(100.0, 100.0)
      .on_click({
        let parent_clicks = parent_clicks.clone();
        move |_| parent_clicks.update(|count| *count += 1)
      })
      .child(Rect::new(40.0, 40.0).background("#22c55e").on_click({
        let child_clicks = child_clicks.clone();
        move |_| child_clicks.update(|count| *count += 1)
      })),
  );
  run_pass(&mut runtime);

  runtime.mouse_down(20.0, 20.0, MouseButton::Left);
  runtime.mouse_up(80.0, 80.0, MouseButton::Left);

  assert_eq!(parent_clicks.get(), 1);
  assert_eq!(child_clicks.get(), 0);
}

#[test]
fn parent_press_child_release_clicks_parent_not_child() {
  let parent_clicks = Signal::new(0);
  let child_clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    Column::new()
      .size(100.0, 100.0)
      .on_click({
        let parent_clicks = parent_clicks.clone();
        move |_| parent_clicks.update(|count| *count += 1)
      })
      .child(Rect::new(40.0, 40.0).background("#22c55e").on_click({
        let child_clicks = child_clicks.clone();
        move |_| child_clicks.update(|count| *count += 1)
      })),
  );
  run_pass(&mut runtime);

  runtime.mouse_down(80.0, 80.0, MouseButton::Left);
  runtime.mouse_up(20.0, 20.0, MouseButton::Left);

  assert_eq!(parent_clicks.get(), 1);
  assert_eq!(child_clicks.get(), 0);
}

#[test]
fn stack_top_child_occludes_lower_sibling_clicks() {
  let lower_clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    Stack::new()
      .child(Rect::new(100.0, 100.0).background("#111827").on_click({
        let lower_clicks = lower_clicks.clone();
        move |_| lower_clicks.update(|count| *count += 1)
      }))
      .child(Rect::new(50.0, 50.0).background("#ffffff")),
  );
  run_pass(&mut runtime);

  runtime.mouse_down(25.0, 25.0, MouseButton::Left);
  runtime.mouse_up(25.0, 25.0, MouseButton::Left);

  assert_eq!(lower_clicks.get(), 0);

  runtime.mouse_down(75.0, 75.0, MouseButton::Left);
  runtime.mouse_up(75.0, 75.0, MouseButton::Left);

  assert_eq!(lower_clicks.get(), 1);
}

#[test]
fn content_only_stack_child_does_not_occlude_lower_sibling_when_empty() {
  let lower_clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    Stack::new()
      .child(Rect::new(100.0, 100.0).background("#111827").on_click({
        let lower_clicks = lower_clicks.clone();
        move |_| lower_clicks.update(|count| *count += 1)
      }))
      .child(Stack::new().size(100.0, 100.0).hit_test(HitTestBehavior::ContentOnly)),
  );
  run_pass(&mut runtime);

  runtime.mouse_down(25.0, 25.0, MouseButton::Left);
  runtime.mouse_up(25.0, 25.0, MouseButton::Left);

  assert_eq!(lower_clicks.get(), 1);
}

#[test]
fn child_of_content_only_wrapper_receives_click_and_occludes_lower_sibling() {
  let lower_clicks = Signal::new(0);
  let child_clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    Stack::new()
      .child(Rect::new(100.0, 100.0).background("#111827").on_click({
        let lower_clicks = lower_clicks.clone();
        move |_| lower_clicks.update(|count| *count += 1)
      }))
      .child(
        Stack::new()
          .size(100.0, 100.0)
          .hit_test(HitTestBehavior::ContentOnly)
          .child(Rect::new(40.0, 40.0).background("#ffffff").on_click({
            let child_clicks = child_clicks.clone();
            move |_| child_clicks.update(|count| *count += 1)
          })),
      ),
  );
  run_pass(&mut runtime);

  runtime.mouse_down(20.0, 20.0, MouseButton::Left);
  runtime.mouse_up(20.0, 20.0, MouseButton::Left);

  assert_eq!(child_clicks.get(), 1);
  assert_eq!(lower_clicks.get(), 0);
}

#[test]
fn pointer_events_none_ignores_node_and_children() {
  let lower_clicks = Signal::new(0);
  let child_clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    Stack::new()
      .child(Rect::new(100.0, 100.0).background("#111827").on_click({
        let lower_clicks = lower_clicks.clone();
        move |_| lower_clicks.update(|count| *count += 1)
      }))
      .child(Stack::new().size(100.0, 100.0).pointer_events_none().child(
        Rect::new(40.0, 40.0).background("#ffffff").on_click({
          let child_clicks = child_clicks.clone();
          move |_| child_clicks.update(|count| *count += 1)
        }),
      )),
  );
  run_pass(&mut runtime);

  runtime.mouse_down(20.0, 20.0, MouseButton::Left);
  runtime.mouse_up(20.0, 20.0, MouseButton::Left);

  assert_eq!(child_clicks.get(), 0);
  assert_eq!(lower_clicks.get(), 1);
}

#[test]
fn rebuilt_descendant_at_same_position_receives_click() {
  let clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(Column::new().child(Rect::new(100.0, 40.0).background("#22c55e")));
  run_pass(&mut runtime);
  let rect = runtime
    .find_element(|element| element.color().is_some())
    .unwrap()
    .bounds();
  let (x, y) = rect.center();

  runtime.mouse_down(x, y, MouseButton::Left);
  runtime.set_root(Column::new().child(Stack::new().size(100.0, 40.0).on_click({
    let clicks = clicks.clone();
    move |_| clicks.update(|count| *count += 1)
  })));
  run_pass(&mut runtime);
  runtime.mouse_up(x, y, MouseButton::Left);

  assert_eq!(clicks.get(), 1);
}
