use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, Ordering},
};

use super::{
  Window, WindowCommand,
  resize::{NativeWindow, ResizeOutcome, ResizeReport, ResizeRestore, SizeReporter, WindowModes, resize_native},
  simulated_window::{MAXIMIZED_CLIENT, MINIMIZED_CLIENT, SimulatedWindow},
};

const MINIMIZED: WindowModes = WindowModes {
  minimized: true,
  maximized: false,
  full_screen: false,
};
const MAXIMIZED: WindowModes = WindowModes {
  minimized: false,
  maximized: true,
  full_screen: false,
};
const FULL_SCREEN: WindowModes = WindowModes {
  minimized: false,
  maximized: false,
  full_screen: true,
};
const NORMAL: WindowModes = WindowModes {
  minimized: false,
  maximized: false,
  full_screen: false,
};

type Outcomes = Arc<Mutex<Vec<(&'static str, Option<(u32, u32)>)>>>;

fn recording(outcomes: &Outcomes, label: &'static str) -> ResizeReport {
  let outcomes = outcomes.clone();
  ResizeReport::new(
    move |outcome| outcomes.lock().unwrap().push((label, outcome.map(|o| o.size))),
    || false,
  )
}

/// Applies the window's queued resizes to `native`, as the winit shell does.
fn apply_queued(window: &Window, native: &SimulatedWindow) {
  for command in window.take_commands() {
    let WindowCommand::Resize(request) = command else {
      panic!("expected a resize, got {command:?}");
    };
    window.apply_resize(Some(native), request);
  }
}

#[test]
fn every_resize_restores_a_minimized_window_before_sizing_it() {
  for restore in [ResizeRestore::Minimized, ResizeRestore::Every] {
    let window = SimulatedWindow::new((1440, 1020), MINIMIZED);

    let outcome = resize_native(&window, 800, 600, restore);

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
}

#[test]
fn an_app_resize_leaves_a_maximized_or_full_screen_window_in_its_mode() {
  for modes in [MAXIMIZED, FULL_SCREEN] {
    let window = SimulatedWindow::new((1440, 1020), modes);
    let outcome = resize_native(&window, 800, 600, ResizeRestore::Minimized);
    assert_eq!(window.calls(), ["request_inner_size(800x600)"]);
    assert_eq!(outcome.left, NORMAL);
    assert_eq!(outcome.remaining, modes, "still in its mode");
  }

  // Through the handle, as an app resizes.
  let window = Window::new();
  window.attach_shell(Arc::new(|| {}));
  let native = SimulatedWindow::new((1440, 1020), MAXIMIZED);
  window.handle().resize(800, 600);
  apply_queued(&window, &native);
  assert_eq!(native.modes(), MAXIMIZED);
  assert_eq!(native.inner_size(), MAXIMIZED_CLIENT);

  let native = SimulatedWindow::new((1440, 1020), MINIMIZED);
  window.handle().resize(800, 600);
  apply_queued(&window, &native);
  assert_eq!(native.modes(), NORMAL, "a minimized window is restored");
  assert_eq!(native.inner_size(), (800, 600), "and takes the size");
}

#[test]
fn an_exact_resize_leaves_full_screen_and_a_maximize_found_under_a_minimize() {
  let window = SimulatedWindow::new((1440, 1020), FULL_SCREEN);
  assert_eq!(resize_native(&window, 800, 600, ResizeRestore::Every).size, (800, 600));
  assert_eq!(window.calls(), ["leave_full_screen", "request_inner_size(800x600)"]);

  let window = SimulatedWindow::new((1440, 1020), MINIMIZED).maximized_under_minimized();
  let outcome = resize_native(&window, 800, 600, ResizeRestore::Every);
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
  let outcome = resize_native(&window, 800, 600, ResizeRestore::Every);
  assert_eq!(window.calls(), ["request_inner_size(800x600)"]);
  assert_eq!(outcome.left, NORMAL);
}

#[test]
fn a_window_the_platform_keeps_minimized_reports_its_minimized_size() {
  let window = SimulatedWindow::new((1440, 1020), MINIMIZED).refusing_restore();
  let outcome = resize_native(&window, 800, 600, ResizeRestore::Every);
  assert_eq!(outcome.size, MINIMIZED_CLIENT);
  assert_eq!(outcome.remaining, MINIMIZED);
}

#[test]
fn each_reported_resize_gets_its_own_outcome_and_app_resizes_wait_for_none() {
  let window = Window::new();
  window.attach_shell(Arc::new(|| {}));
  let outcomes = Outcomes::default();
  window.resize_reported(800, 600, recording(&outcomes, "first"));
  window.resize_reported(1024, 600, recording(&outcomes, "second"));
  window.handle().resize(640, 480);

  let native = SimulatedWindow::new((1440, 1020), MINIMIZED);
  let commands = window.take_commands();
  assert_eq!(commands.len(), 3);
  for command in commands.into_iter().rev() {
    let WindowCommand::Resize(request) = command else {
      panic!("expected a resize, got {command:?}");
    };
    window.apply_resize(Some(&native), request);
  }
  assert_eq!(
    *outcomes.lock().unwrap(),
    [("second", Some((1024, 600))), ("first", Some((800, 600)))]
  );
  assert_eq!(window.pending_resize_reports(), 0);

  window.resize_reported(800, 600, recording(&outcomes, "closed"));
  let Some(WindowCommand::Resize(request)) = window.take_commands().pop() else {
    panic!("expected a resize");
  };
  window.apply_resize(None::<&SimulatedWindow>, request);
  assert_eq!(outcomes.lock().unwrap().last(), Some(&("closed", None)));
}

#[test]
fn reports_whose_command_was_discarded_or_whose_caller_left_are_dropped() {
  let window = Window::new();
  window.attach_shell(Arc::new(|| {}));
  let outcomes = Outcomes::default();

  // Its command leaves the queue without being applied.
  window.resize_reported(800, 600, recording(&outcomes, "discarded"));
  window.take_commands();
  // Its caller stops waiting (an MCP call that timed out).
  let gone = Arc::new(AtomicBool::new(false));
  window.resize_reported(
    800,
    600,
    ResizeReport::new(|_| panic!("nobody waits for this report"), {
      let gone = gone.clone();
      move || gone.load(Ordering::SeqCst)
    }),
  );
  assert_eq!(
    window.pending_resize_reports(),
    1,
    "the discarded report went when the next was queued"
  );
  gone.store(true, Ordering::SeqCst);
  window.take_commands();

  window.resize_reported(1024, 600, recording(&outcomes, "kept"));
  assert_eq!(window.pending_resize_reports(), 1, "only the last report waits");
  apply_queued(&window, &SimulatedWindow::new((1440, 1020), NORMAL));
  assert_eq!(*outcomes.lock().unwrap(), [("kept", Some((1024, 600)))]);
}

#[test]
fn a_report_whose_caller_left_is_dropped_while_its_command_waits() {
  let window = Window::new();
  window.attach_shell(Arc::new(|| {}));
  let gone = Arc::new(AtomicBool::new(true));
  window.resize_reported(
    800,
    600,
    ResizeReport::new(|_| panic!("nobody waits for this report"), {
      let gone = gone.clone();
      move || gone.load(Ordering::SeqCst)
    }),
  );
  let outcomes = Outcomes::default();
  window.resize_reported(1024, 600, recording(&outcomes, "kept"));
  assert_eq!(window.pending_resize_reports(), 1);
  // The abandoned report's command still applies; only its report is gone.
  apply_queued(&window, &SimulatedWindow::new((1440, 1020), NORMAL));
  assert_eq!(*outcomes.lock().unwrap(), [("kept", Some((1024, 600)))]);
}

#[test]
fn a_headless_window_reports_no_native_window_at_once_and_queues_nothing() {
  let window = Window::new();
  let outcomes = Outcomes::default();
  window.resize_reported(800, 600, recording(&outcomes, "headless"));
  assert_eq!(*outcomes.lock().unwrap(), [("headless", None)]);
  assert!(window.take_commands().is_empty());
  assert_eq!(window.pending_resize_reports(), 0);
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
