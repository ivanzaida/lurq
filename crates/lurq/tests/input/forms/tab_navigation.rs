use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::events::MouseButton,
  components::{Button, Column, Form, FormHandle, FormOptions, FormProps, TextInput},
  core::Signal,
  node::color::Color,
};

use crate::support::{pointer_click, run_pass};

#[test]
fn tab_moves_focus_through_form_controls_in_order() {
  let first_focus = Arc::new(AtomicUsize::new(0));
  let second_focus = Arc::new(AtomicUsize::new(0));
  let button_focus = Arc::new(AtomicUsize::new(0));
  let mut runtime = lurq::app::Tree::new();

  runtime.set_root(Form::element(
    FormProps::new(FormHandle::new(FormOptions::new())),
    Column::new()
      .child(TextInput::new(Signal::new(String::new())).on_focus({
        let first_focus = first_focus.clone();
        move || {
          first_focus.fetch_add(1, Ordering::SeqCst);
        }
      }))
      .child(TextInput::new(Signal::new(String::new())).on_focus({
        let second_focus = second_focus.clone();
        move || {
          second_focus.fetch_add(1, Ordering::SeqCst);
        }
      }))
      .child(Button::new("Save").on_focus({
        let button_focus = button_focus.clone();
        move || {
          button_focus.fetch_add(1, Ordering::SeqCst);
        }
      })),
  ));
  run_pass(&mut runtime);

  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);

  assert_eq!(first_focus.load(Ordering::SeqCst), 1);
  assert_eq!(second_focus.load(Ordering::SeqCst), 1);
  assert_eq!(button_focus.load(Ordering::SeqCst), 1);
}

#[test]
fn shift_tab_moves_focus_backward_in_form_scope() {
  let first_focus = Arc::new(AtomicUsize::new(0));
  let button_focus = Arc::new(AtomicUsize::new(0));
  let mut runtime = lurq::app::Tree::new();

  runtime.set_root(Form::element(
    FormProps::new(FormHandle::new(FormOptions::new())),
    Column::new()
      .child(TextInput::new(Signal::new(String::new())).on_focus({
        let first_focus = first_focus.clone();
        move || {
          first_focus.fetch_add(1, Ordering::SeqCst);
        }
      }))
      .child(Button::new("Save").on_focus({
        let button_focus = button_focus.clone();
        move || {
          button_focus.fetch_add(1, Ordering::SeqCst);
        }
      })),
  ));
  run_pass(&mut runtime);

  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), true, false, false);
  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), true, false, false);

  assert_eq!(button_focus.load(Ordering::SeqCst), 1);
  assert_eq!(first_focus.load(Ordering::SeqCst), 1);
}

#[test]
fn tab_focus_fires_handler_attached_to_styled_input_wrapper() {
  let focus = Arc::new(AtomicUsize::new(0));
  let mut runtime = lurq::app::Tree::new();

  runtime.set_root(Form::element(
    FormProps::new(FormHandle::new(FormOptions::new())),
    Column::new().child(TextInput::new(Signal::new(String::new())).width(100.0).on_focus({
      let focus = focus.clone();
      move || {
        focus.fetch_add(1, Ordering::SeqCst);
      }
    })),
  ));
  run_pass(&mut runtime);

  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);

  assert_eq!(focus.load(Ordering::SeqCst), 1);
}

#[test]
fn tab_does_not_focus_controls_outside_form() {
  let focus = Arc::new(AtomicUsize::new(0));
  let mut runtime = lurq::app::Tree::new();

  runtime.set_root(
    Column::new().child(TextInput::new(Signal::new(String::new())).on_focus({
      let focus = focus.clone();
      move || {
        focus.fetch_add(1, Ordering::SeqCst);
      }
    })),
  );
  run_pass(&mut runtime);

  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);

  assert_eq!(focus.load(Ordering::SeqCst), 0);
}

#[test]
fn tab_from_focused_control_outside_form_does_not_enter_form() {
  let outside_focus = Arc::new(AtomicUsize::new(0));
  let form_focus = Arc::new(AtomicUsize::new(0));
  let mut runtime = lurq::app::Tree::new();

  runtime.set_root(
    Column::new()
      .child(
        TextInput::new(Signal::new(String::new()))
          .width(100.0)
          .background("#ef4444")
          .on_focus({
            let outside_focus = outside_focus.clone();
            move || {
              outside_focus.fetch_add(1, Ordering::SeqCst);
            }
          }),
      )
      .child(Form::element(
        FormProps::new(FormHandle::new(FormOptions::new())),
        Column::new().child(TextInput::new(Signal::new(String::new())).on_focus({
          let form_focus = form_focus.clone();
          move || {
            form_focus.fetch_add(1, Ordering::SeqCst);
          }
        })),
      )),
  );
  run_pass(&mut runtime);

  let outside = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .expect("outside input should render")
    .bounds();
  pointer_click(
    &mut runtime,
    outside.x + outside.width / 2.0,
    outside.y + outside.height / 2.0,
    MouseButton::Left,
  );
  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);

  assert_eq!(outside_focus.load(Ordering::SeqCst), 1);
  assert_eq!(form_focus.load(Ordering::SeqCst), 0);
}

#[test]
fn tab_treats_styled_button_as_one_focus_stop() {
  let clear_focus = Arc::new(AtomicUsize::new(0));
  let submit_focus = Arc::new(AtomicUsize::new(0));
  let mut runtime = lurq::app::Tree::new();

  runtime.set_root(Form::element(
    FormProps::new(FormHandle::new(FormOptions::new())),
    Column::new().child(
      lurq::components::Row::new()
        .child(
          Button::new("Clear")
            .height(34.0)
            .padding_horizontal(14.0)
            .button()
            .on_focus({
              let clear_focus = clear_focus.clone();
              move || {
                clear_focus.fetch_add(1, Ordering::SeqCst);
              }
            }),
        )
        .child(
          Button::new("Submit")
            .height(34.0)
            .padding_horizontal(14.0)
            .submit()
            .on_focus({
              let submit_focus = submit_focus.clone();
              move || {
                submit_focus.fetch_add(1, Ordering::SeqCst);
              }
            }),
        ),
    ),
  ));
  run_pass(&mut runtime);

  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);

  assert_eq!(clear_focus.load(Ordering::SeqCst), 1);
  assert_eq!(submit_focus.load(Ordering::SeqCst), 1);
}
