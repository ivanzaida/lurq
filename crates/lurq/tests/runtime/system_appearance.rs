//! The root is created and first rendered in the system's look where the system said it before the root was
//! mounted: the app asked it (`App::read_system_appearance`), or a shell heard it for a window before.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    theme::PaletteColor,
    window::{Appearance, SystemSettings},
  },
  components::Column,
  node::Element,
};

/// What the root saw, in order.
#[derive(Clone, Default)]
struct Seen(Arc<Mutex<Vec<String>>>);

// Props of a component are inspectable by DevTools in a build with it; this test needs nothing shown.
impl lurq::app::component::DevtoolsInspectable for Seen {}

impl PartialEq for Seen {
  fn eq(&self, _: &Self) -> bool {
    true
  }
}

/// How a component reads the look it is in: the theme's appearance, the window's, and the background's color.
fn look(ctx: &mut Ctx) -> String {
  let theme = ctx.theme().clone();
  let dark = theme.palette_color(PaletteColor::Background)
    == theme.palette_color_for(Appearance::Dark, PaletteColor::Background);
  format!(
    "theme {:?}, window {:?}, a {} background",
    theme.appearance(),
    ctx.window().info().system.appearance,
    if dark { "dark" } else { "light" }
  )
}

struct Root {
  seen: Seen,
}

impl Component for Root {
  type Props = Seen;

  fn create(ctx: &mut Ctx) -> Self {
    let seen = ctx.props::<Seen>().clone();
    let look = look(ctx);
    seen.0.lock().unwrap().push(format!("create: {look}"));
    Self { seen }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let look = look(ctx);
    self.seen.0.lock().unwrap().push(format!("render: {look}"));
    Column::new()
  }
}

fn mounted(app: &mut App) -> Vec<String> {
  let seen = Seen::default();
  let mut tree = Tree::new();
  tree.resize(200, 200);
  tree.mount_root::<Root>(app, seen.clone());
  let seen = seen.0.lock().unwrap().clone();
  seen
}

#[test]
fn a_root_mounted_before_the_system_said_anything_is_in_the_light_look() {
  // An app asks nothing by itself: a test is in the light look on every machine.
  let mut app = App::new();
  assert_eq!(
    mounted(&mut app),
    [
      "create: theme Light, window Light, a light background",
      "render: theme Light, window Light, a light background",
    ]
  );
}

#[test]
fn a_root_mounted_after_the_system_said_dark_is_created_in_the_dark_look() {
  let mut app = App::new();
  // What `App::read_system_appearance` does with what a dark system says.
  app.theme().apply_system_appearance(Appearance::Dark);
  assert_eq!(
    mounted(&mut app),
    [
      "create: theme Dark, window Dark, a dark background",
      "render: theme Dark, window Dark, a dark background",
    ]
  );

  // And what a shell told the theme for a window before: a second window's root is in it from its create.
  let mut app = App::new();
  app.theme().apply_system(&SystemSettings {
    appearance: Appearance::Dark,
    ..SystemSettings::default()
  });
  assert_eq!(
    mounted(&mut app)[0],
    "create: theme Dark, window Dark, a dark background"
  );
}

#[test]
fn a_theme_with_a_look_of_its_own_keeps_it() {
  // The app's own "Light" setting, on a dark system.
  let mut app = App::new();
  app.theme().set_preferred_appearance(Some(Appearance::Light));
  app.theme().apply_system_appearance(Appearance::Dark);
  assert_eq!(
    mounted(&mut app)[0],
    "create: theme Light, window Dark, a light background",
    "the window still says what the system is in"
  );

  // A theme that does not follow the system.
  let mut app = App::new();
  app.theme().set_follows_system(false);
  app.theme().apply_system_appearance(Appearance::Dark);
  assert_eq!(
    mounted(&mut app)[0],
    "create: theme Light, window Dark, a light background"
  );
}

#[test]
fn reading_the_system_gives_the_theme_what_it_said() {
  let app = App::new();
  match app.read_system_appearance() {
    // Whatever this machine is in, the theme is in it too.
    Some(appearance) => assert_eq!(app.theme().appearance(), appearance),
    // A system that cannot be asked with no window changes nothing.
    None => assert_eq!(app.theme().appearance(), Appearance::Light),
  }
  #[cfg(windows)]
  assert!(app.read_system_appearance().is_some(), "Windows says it with no window");
}
