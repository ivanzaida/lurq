use std::time::{Duration, Instant};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, FutureStatus, Timeout},
  },
  components::Text,
  core::Signal,
  layout::text_style::TextStyle,
  node::Element,
};

use crate::support::TestSurface;

fn app_with_size(size: f32) -> App {
  let app = App::new();
  app.theme().set_default_text_style(TextStyle {
    font_size: size,
    ..TextStyle::default()
  });
  app
}

struct TimerApp {
  beat: Signal<u32>,
  _timer: Timeout,
}
impl Component for TimerApp {
  type Props = ();
  fn create(ctx: &mut Ctx) -> Self {
    let beat = ctx.signal(0);
    let update = beat.clone();
    let timer = ctx.create_timeout(Duration::from_secs(1), move || update.set(1));
    timer.start();
    Self { beat, _timer: timer }
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let beat = self.beat.get();
    if beat > 0 {
      ctx
        .app_ref_mut()
        .install_fonts([vec![0; 16]], std::iter::empty::<(&str, &str)>());
    }
    Text::new(&format!(
      "{}:{beat}",
      ctx.app_ref().theme().default_text_style().font_size
    ))
  }
}

#[inline(never)]
fn mounted_timer() -> (App, Tree) {
  let mut app = app_with_size(37.0);
  let mut tree = Tree::new();
  tree.mount_root::<TimerApp>(&mut app, ());
  tree.pass(&mut app, &TestSurface);
  (app, tree)
}

#[test]
fn timer_can_read_and_write_app_after_it_moves_out_of_mount_frame() {
  let (app, mut tree) = mounted_timer();
  let moved = Box::new(app);
  tree.tick_timers_at(Instant::now() + Duration::from_secs(2));
  assert_eq!(tree.root().unwrap().text_content(), Some("37:1"));
  assert_eq!(moved.theme().default_text_style().font_size, 37.0);
}

#[test]
fn timer_retains_app_services_after_external_owner_is_dropped() {
  let (app, mut tree) = mounted_timer();
  drop(app);
  tree.tick_timers_at(Instant::now() + Duration::from_secs(2));
  assert_eq!(tree.root().unwrap().text_content(), Some("37:1"));
}

struct FutureApp;
impl Component for FutureApp {
  type Props = ();
  fn create(_: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let result = ctx.future((), |_| async { Ok::<_, String>(()) }).state().get();
    if result.status == FutureStatus::Fulfilled {
      ctx
        .app_ref_mut()
        .install_fonts([vec![0; 16]], std::iter::empty::<(&str, &str)>());
      Text::new(&format!(
        "done:{}",
        ctx.app_ref().theme().default_text_style().font_size
      ))
    } else {
      Text::new("pending")
    }
  }
}

#[test]
fn future_can_use_app_after_external_owner_is_dropped() {
  let mut tree = Tree::new();
  tree.mount_root::<FutureApp>(&mut app_with_size(41.0), ());
  tree.tick_futures();
  assert_eq!(tree.root().unwrap().text_content(), Some("done:41"));
}

struct KeyApp {
  count: Signal<u32>,
}
impl Component for KeyApp {
  type Props = ();
  fn create(ctx: &mut Ctx) -> Self {
    Self { count: ctx.signal(0) }
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let update = self.count.clone();
    Text::new(&format!(
      "{}:{}",
      ctx.app_ref().theme().default_text_style().font_size,
      self.count.get()
    ))
    .on_key_down(move |_| update.update(|count| *count += 1))
  }
}

#[test]
fn skipped_pass_rebinds_app_before_a_later_event_render() {
  let mut app = app_with_size(29.0);
  let mut tree = Tree::new();
  tree.mount_root::<KeyApp>(&mut app, ());
  tree.pass(&mut app, &TestSurface);
  let mut replacement = app_with_size(53.0);
  assert!(!tree.pass(&mut replacement, &TestSurface).required);
  drop(app);
  tree.key_down("a".into(), "KeyA".into(), false, false, false);
  assert_eq!(tree.root().unwrap().text_content(), Some("53:1"));
}

#[cfg(feature = "persistent_storage")]
#[test]
fn cloned_app_observes_storage_backend_replacement() {
  let mut app = App::new();
  let clone = app.clone();
  let path = std::env::temp_dir().join(format!("lurq-app-lifetime-{}.redb", std::process::id()));
  app.set_persistent_storage_path(&path).unwrap();
  clone.set_persistent_value("probe", 17u32).unwrap();
  assert_eq!(app.persistent_value::<u32>("probe"), Some(17));
  drop(clone);
  drop(app);
  std::fs::remove_file(path).unwrap();
}

#[cfg(feature = "canvas")]
#[test]
fn canvas_metrics_callback_can_mutate_shared_app_services() {
  let mut app = App::new();
  let retained = app.clone();
  let reference = lurq::core::ElementRef::new();
  let mut tree = Tree::new();
  tree.set_root(
    lurq::components::Canvas::new()
      .ref_element(reference.clone())
      .width(40.)
      .height(40.),
  );
  tree.pass(&mut app, &TestSurface);
  let canvas = reference.as_canvas().unwrap();
  let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
  let callback_calls = calls.clone();
  let _observer = canvas.observe_metrics(move |_| {
    retained
      .clone()
      .install_fonts([vec![0; 16]], std::iter::empty::<(&str, &str)>());
    callback_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
  });
  reference.mutable().set_relative_bounds(0., 0., 80., 80.);
  tree.request_redraw();
  tree.pass(&mut app, &TestSurface);
  assert!(calls.load(std::sync::atomic::Ordering::SeqCst) >= 2);
}

#[test]
fn disappearing_overlay_blur_can_mutate_shared_app_services() {
  use lurq::{
    components::{Column, Overlay, Rect, TextInput},
    core::ElementRef,
  };
  let mut app = App::new();
  let retained = app.clone();
  let anchor = ElementRef::new();
  let input = ElementRef::new();
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .child(Rect::new(100., 30.).ref_element(anchor.clone()))
      .child(
        Overlay::new(
          TextInput::new(Signal::new(String::new()))
            .id("overlay-field")
            .ref_element(input.clone())
            .on_blur(move || {
              retained
                .clone()
                .install_fonts([vec![0; 16]], std::iter::empty::<(&str, &str)>());
            }),
        )
        .anchor(anchor.clone()),
      ),
  );
  tree.pass(&mut app, &TestSurface);
  tree.get_element_by_id_mut("overlay-field").unwrap().focus();
  assert!(input.focused());
  anchor.mutable().set_relative_bounds(0., 0., 0., 0.);
  tree.request_redraw();
  tree.pass(&mut app, &TestSurface);
  assert!(!input.focused());
}
