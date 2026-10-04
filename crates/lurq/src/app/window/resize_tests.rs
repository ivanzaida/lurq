use std::sync::{Arc, Mutex};

use super::{
  Window, WindowCommand,
  resize::{ResizeOutcome, SizeReporter, WindowModes, resize_native},
  simulated_window::{MINIMIZED_CLIENT, SimulatedWindow},
};

const MINIMIZED: WindowModes = WindowModes {
  minimized: true,
  maximized: false,
  full_screen: false,
};
const NORMAL: WindowModes = WindowModes {
  minimized: false,
  maximized: false,
  full_screen: false,
};

#[test]
fn a_resize_restores_a_minimized_window_before_sizing_it() {
  let window = SimulatedWindow::new((1440, 1020), MINIMIZED);

  let outcome = resize_native(&window, 800, 600);

  assert_eq!(window.calls(), ["set_minimized(false)", "request_inner_size(800x600)"]);
  assert_eq!(
    outcome,
    ResizeOutcome {
      requested: (800, 600),
      size: (800, 600),
      left: MINIMIZED,
      remaining: NORMAL,
    }
  );
}

#[test]
fn a_resize_leaves_full_screen_and_a_maximize_found_under_a_minimize() {
  let full_screen = WindowModes {
    full_screen: true,
    ..NORMAL
  };
  let window = SimulatedWindow::new((1440, 1020), full_screen);
  assert_eq!(resize_native(&window, 800, 600).size, (800, 600));
  assert_eq!(window.calls(), ["leave_full_screen", "request_inner_size(800x600)"]);

  let window = SimulatedWindow::new((1440, 1020), MINIMIZED).maximized_under_minimized();
  let outcome = resize_native(&window, 800, 600);
  assert_eq!(
    window.calls(),
    [
      "set_minimized(false)",
      "set_maximized(false)",
      "request_inner_size(800x600)"
    ]
  );
  assert_eq!(outcome.size, (800, 600));
  assert_eq!(outcome.left.names(), ["minimized", "maximized"]);
}

#[test]
fn a_normal_window_is_only_sized() {
  let window = SimulatedWindow::new((1440, 1020), NORMAL);
  let outcome = resize_native(&window, 800, 600);
  assert_eq!(window.calls(), ["request_inner_size(800x600)"]);
  assert_eq!(outcome.left, NORMAL);
}

#[test]
fn a_window_the_platform_keeps_minimized_reports_its_minimized_size() {
  let window = SimulatedWindow::new((1440, 1020), MINIMIZED).refusing_restore();
  let outcome = resize_native(&window, 800, 600);
  assert_eq!(outcome.size, MINIMIZED_CLIENT);
  assert_eq!(outcome.remaining, MINIMIZED);
}

#[test]
fn each_reported_resize_gets_its_own_outcome_and_app_resizes_wait_for_none() {
  let window = Window::new();
  window.attach_shell(Arc::new(|| {}));
  let outcomes = Arc::new(Mutex::new(Vec::new()));
  for label in ["first", "second"] {
    let outcomes = outcomes.clone();
    window.resize_reported(
      if label == "first" { 800 } else { 1024 },
      600,
      Box::new(move |outcome| outcomes.lock().unwrap().push((label, outcome.map(|o| o.size)))),
    );
  }
  window.handle().resize(640, 480);

  let native = SimulatedWindow::new((1440, 1020), MINIMIZED);
  let commands = window.take_commands();
  assert_eq!(commands.len(), 3);
  for command in commands.into_iter().rev() {
    let WindowCommand::Resize { width, height, report } = command else {
      panic!("expected a resize, got {command:?}");
    };
    window.apply_resize(Some(&native), width, height, report);
  }
  assert_eq!(
    *outcomes.lock().unwrap(),
    [("second", Some((1024, 600))), ("first", Some((800, 600)))]
  );

  window.resize_reported(800, 600, {
    let outcomes = outcomes.clone();
    Box::new(move |outcome| outcomes.lock().unwrap().push(("closed", outcome.map(|o| o.size))))
  });
  let Some(WindowCommand::Resize { width, height, report }) = window.take_commands().pop() else {
    panic!("expected a resize");
  };
  window.apply_resize(None::<&SimulatedWindow>, width, height, report);
  assert_eq!(outcomes.lock().unwrap().last(), Some(&("closed", None)));
}

#[test]
fn a_headless_window_reports_no_native_window_at_once_and_queues_nothing() {
  let window = Window::new();
  let outcome = Arc::new(Mutex::new(Some(Some((0, 0)))));
  let slot = outcome.clone();
  window.resize_reported(
    800,
    600,
    Box::new(move |result| *slot.lock().unwrap() = Some(result.map(|o| o.size))),
  );
  assert_eq!(*outcome.lock().unwrap(), Some(None));
  assert!(window.take_commands().is_empty());
}

#[test]
fn a_minimized_size_is_never_reported_and_a_size_is_reported_once() {
  let mut reporter = SizeReporter::default();
  assert_eq!(reporter.next(true, (158, 26)), None);
  assert_eq!(reporter.next(false, (960, 680)), Some((960, 680)));
  assert_eq!(reporter.next(false, (960, 680)), None);
  assert_eq!(reporter.next(true, (158, 26)), None);
  assert_eq!(reporter.next(false, (960, 680)), None);
  assert_eq!(reporter.next(false, (800, 600)), Some((800, 600)));
}
