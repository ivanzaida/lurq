//! Main-thread AppKit integration test (the ordinary Rust test harness runs
//! tests on worker threads). Runs in CI on macOS; other hosts do nothing.
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
  macos::run();
}

#[cfg(target_os = "macos")]
mod macos {
  use std::{
    sync::{
      Arc, Mutex, OnceLock,
      atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
  };

  use lurq::{
    app::{
      Accelerator, App, CloseRequest, CloseRequestSource, Menu, MenuAction, MenuBar, Tree, WindowHandle,
      component::Component, ctx::Ctx, winit_shell::WinitWindow,
    },
    components::Text,
    node::Element,
  };
  use objc2_app_kit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventType};
  use objc2_foundation::{MainThreadMarker, NSPoint, NSString};

  #[derive(Default)]
  struct Shared {
    window: Mutex<Option<WindowHandle>>,
    pending: Mutex<Option<CloseRequest>>,
    closes: AtomicUsize,
    keys: AtomicUsize,
    activations: AtomicUsize,
  }
  static SHARED: OnceLock<Arc<Shared>> = OnceLock::new();
  struct Root;
  impl Component for Root {
    type Props = ();
    fn create(ctx: &mut Ctx) -> Self {
      let shared = SHARED.get().unwrap().clone();
      *shared.window.lock().unwrap() = Some(ctx.window());
      ctx.window().on_close_requested(move |r| {
        shared.closes.fetch_add(1, Ordering::SeqCst);
        *shared.pending.lock().unwrap() = Some(r);
      });
      Self
    }
    fn render(&self, _: &mut Ctx) -> impl Into<Element> {
      let down = SHARED.get().unwrap().clone();
      let up = down.clone();
      Text::new("Native lifecycle integration")
        .on_key_down(move |_| {
          down.keys.fetch_add(1, Ordering::SeqCst);
        })
        .on_key_up(move |_| {
          up.keys.fetch_add(1, Ordering::SeqCst);
        })
    }
  }
  fn bar(enabled: bool) -> MenuBar {
    MenuBar {
      menus: vec![Menu {
        title: "File".into(),
        items: vec![
          MenuAction::new("new", if enabled { "New" } else { "Nouveau" })
            .accelerator(Accelerator::command("n"))
            .enabled(enabled)
            .into(),
        ],
      }],
      ..Default::default()
    }
  }
  fn send_key(app: &NSApplication, key: &str, code: u16) {
    let chars = NSString::from_str(key);
    for kind in [NSEventType::KeyDown, NSEventType::KeyUp] {
      let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
        kind, NSPoint::new(0.0, 0.0), NSEventModifierFlags::Command, 0.0,
        app.windows().objectAtIndex(0).windowNumber(), None, &chars, &chars, false, code).unwrap();
      app.sendEvent(&event);
    }
  }
  pub fn run() {
    let shared = Arc::new(Shared::default());
    assert!(SHARED.set(shared.clone()).is_ok());
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.mount_root::<Root>(&mut app, ());
    app.set_menu_bar(bar(true));
    let controller = app.menu_controller();
    let callback = shared.clone();
    app.on_menu_activate(move |id| {
      eprintln!("native activation: {id}");
      if id == "quit" {
        callback.window.lock().unwrap().as_ref().unwrap().request_close();
      } else {
        assert_eq!(id, "new");
        callback.activations.fetch_add(1, Ordering::SeqCst);
      }
    });
    let check = shared.clone();
    let mut phase = 0;
    let mut last = Instant::now();
    let started = Instant::now();
    WinitWindow::new(app, tree)
      .with_title("lurq native integration")
      .with_size(300, 180)
      .on_tick(move |_, _| {
        assert!(
          started.elapsed() < Duration::from_secs(15),
          "native integration timed out"
        );
        if last.elapsed() < Duration::from_millis(60) {
          return;
        }
        last = Instant::now();
        let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
        let file = app.mainMenu().unwrap().itemAtIndex(1).unwrap().submenu().unwrap();
        eprintln!(
          "native phase={phase} closes={} activations={}",
          check.closes.load(Ordering::SeqCst),
          check.activations.load(Ordering::SeqCst)
        );
        if matches!(phase, 4 | 5) {
          let application = app.mainMenu().unwrap().itemAtIndex(0).unwrap().submenu().unwrap();
          for index in 0..application.numberOfItems() {
            let item = application.itemAtIndex(index).unwrap();
            eprintln!(
              "application item={} enabled={} key={} modifiers={:?}",
              item.title(),
              item.isEnabled(),
              item.keyEquivalent(),
              item.keyEquivalentModifierMask()
            );
          }
        }
        match phase {
          0 => file.performActionForItemAtIndex(0),
          1 => {
            assert_eq!(check.activations.load(Ordering::SeqCst), 1);
            send_key(&app, "n", 45);
          }
          2 => {
            assert_eq!(check.activations.load(Ordering::SeqCst), 2);
            controller.set(bar(false));
          }
          3 => {
            let item = file.itemAtIndex(0).unwrap();
            assert_eq!(item.title().to_string(), "Nouveau");
            assert!(!item.isEnabled());
            file.performActionForItemAtIndex(0);
            // Dock Quit uses this NSApplication termination path.
            app.terminate(None);
          }
          4 => {
            assert_eq!(check.activations.load(Ordering::SeqCst), 2);
            assert_eq!(check.closes.load(Ordering::SeqCst), 1);
            let request = check.pending.lock().unwrap().take().unwrap();
            assert_eq!(request.source(), CloseRequestSource::Os);
            request.cancel();
            send_key(&app, "q", 12);
          }
          5 => {
            if check.closes.load(Ordering::SeqCst) < 2 {
              return;
            }
            assert_eq!(check.closes.load(Ordering::SeqCst), 2);
            check.pending.lock().unwrap().take().unwrap().cancel();
            // Native title-bar close must reach the same handler.
            app.windows().objectAtIndex(0).performClose(None);
          }
          6 => {
            assert_eq!(check.closes.load(Ordering::SeqCst), 3);
            assert_eq!(
              check.pending.lock().unwrap().as_ref().unwrap().source(),
              CloseRequestSource::Os
            );
            // Retain the decision until a later event-loop turn.
          }
          7 => check.pending.lock().unwrap().take().unwrap().proceed(),
          _ => panic!("proceed did not exit the loop"),
        }
        phase += 1;
      })
      .run();
    assert_eq!(shared.closes.load(Ordering::SeqCst), 3);
    assert_eq!(shared.activations.load(Ordering::SeqCst), 2);
    assert_eq!(shared.keys.load(Ordering::SeqCst), 0);
    println!(
      "native macOS menu click, accelerator, update, Dock Quit, Cmd+Q, native close, cancel and delayed proceed passed"
    );
  }
}
