//! A pager (`components::Pager`): pages one at a time, of which the one shown and the ones beside it are built.
//! The index is the truth: the pager shows the page the app's signal says, and a person's move changes the
//! signal once the page is at rest. A finger, a wheel and the keys turn it a page.
//!
//! The fingers are given through [`Tree::finger`] on the tree's clock of presses, as a shell gives them, and
//! what the wheel and the app's own index animate is stepped on the tree's clock of animations.

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::{ScrollPhase, TouchPhase},
  },
  components::{Column, Pager, TextInput},
  core::Signal,
  node::Element,
};

use crate::support::TestSurface;

const FINGER: u64 = 1;
/// A page, which is as large as the pager.
const WIDTH: f32 = 100.0;
const HEIGHT: f32 = 200.0;

/// What the pages and the pager told, in order.
#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
  fn say(&self, line: String) {
    self.0.lock().unwrap().push(line);
  }

  fn take(&self) -> Vec<String> {
    std::mem::take(&mut *self.0.lock().unwrap())
  }
}

struct LeafProps {
  page: usize,
  log: Log,
}

impl PartialEq for LeafProps {
  fn eq(&self, other: &Self) -> bool {
    self.page == other.page
  }
}

impl lurq::app::component::DevtoolsInspectable for LeafProps {}

/// A page with a life of its own: it tells when it is made and when it is dropped.
struct Leaf {
  page: usize,
  log: Log,
}

impl Component for Leaf {
  type Props = LeafProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<LeafProps>();
    let (page, log) = (props.page, props.log.clone());
    log.say(format!("made {page}"));
    Self { page, log }
  }

  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    Column::new().id(format!("leaf-{}", self.page))
  }

  fn on_unmounted(&self) {
    self.log.say(format!("dropped {}", self.page));
  }
}

/// What a scene shows: built anew by each of its renders.
struct SceneProps(Arc<dyn Fn() -> Element + Send + Sync>);

impl PartialEq for SceneProps {
  fn eq(&self, _: &Self) -> bool {
    true
  }
}

impl lurq::app::component::DevtoolsInspectable for SceneProps {}

/// The root of a test's tree: a pager in a tree is mounted by the component that renders the tree.
struct Scene;

impl Component for Scene {
  type Props = SceneProps;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let build = ctx.props::<SceneProps>().0.clone();
    build()
  }
}

struct Stage {
  app: App,
  tree: Tree,
  /// The page the pager is at, which the app holds.
  index: Signal<usize>,
  log: Log,
  count: usize,
  now: Instant,
}

impl Stage {
  /// A pager of `count` pages of 100 by 200 at the corner of the window, down or across.
  fn new(count: usize, across: bool) -> Self {
    Self::with(count, across, |pager| pager)
  }

  fn with(count: usize, across: bool, more: impl FnOnce(Pager) -> Pager) -> Self {
    let (index, log) = (Signal::new(0), Log::default());
    let (pages, told) = (log.clone(), log.clone());
    let mut pager = Pager::new(count, move |ctx: &mut Ctx, page| {
      ctx.mount::<Leaf>(LeafProps {
        page,
        log: pages.clone(),
      })
    })
    .index(index.clone())
    .on_index_change(move |to| told.say(format!("index {to}")))
    .id("pager");
    if across {
      pager = pager.horizontal();
    }
    let pager = more(pager);
    let field = Signal::new(String::new());
    Self::of(
      count,
      index,
      log,
      Arc::new(move || {
        Column::new()
          .size(300.0, 300.0)
          .child(Column::new().size(WIDTH, HEIGHT).child(pager.clone()))
          .child(TextInput::new(field.clone()).id("field").width(100.0))
          .into()
      }),
    )
  }

  /// A stage that shows what `scene` builds.
  fn of(count: usize, index: Signal<usize>, log: Log, scene: Arc<dyn Fn() -> Element + Send + Sync>) -> Self {
    let mut stage = Self {
      app: App::new(),
      tree: Tree::new(),
      index,
      log,
      count,
      now: Instant::now(),
    };
    stage.tree.resize(300, 300);
    stage.tree.mount_root::<Scene>(&mut stage.app, SceneProps(scene));
    stage.tree.set_press_clock(Some(stage.now));
    stage.tree.set_animation_clock(Some(stage.now));
    // Its room is known once it is laid out, and its pages are as large in the pass after; a pager in a page
    // of another is laid out after that one.
    for _ in 0..6 {
      stage.pass();
    }
    stage
  }

  fn pass(&mut self) {
    self.tree.pass(&mut self.app, &TestSurface);
  }

  /// Time goes on by `millis`, a step every 8 ms: what a finger left and what is animated go on with it.
  fn run(&mut self, millis: u64) {
    let until = self.now + Duration::from_millis(millis);
    while self.now < until {
      self.now += Duration::from_millis(8);
      self.tree.set_animation_clock(Some(self.now));
      self.tree.set_press_clock(Some(self.now));
      self.tree.finger_tick(self.now);
      self.pass();
    }
  }

  fn finger(&mut self, millis: u64, phase: TouchPhase, x: f32, y: f32) {
    self.now += Duration::from_millis(millis);
    self.tree.set_press_clock(Some(self.now));
    self.tree.finger(FINGER, phase, x, y, None);
  }

  /// A finger that lands at `from` and goes `(dx, dy)` in three moves 8 ms apart, and lifts at once: a flick.
  fn flick(&mut self, from: (f32, f32), (dx, dy): (f32, f32)) {
    self.finger(0, TouchPhase::Start, from.0, from.1);
    for step in 1..=3 {
      let share = step as f32 / 3.0;
      self.finger(8, TouchPhase::Move, from.0 + dx * share, from.1 + dy * share);
    }
    self.finger(4, TouchPhase::End, from.0 + dx, from.1 + dy);
    self.pass();
  }

  /// The pages that are built now.
  fn built(&mut self) -> Vec<usize> {
    (0..self.count)
      .filter(|page| self.tree.get_element_by_id(&format!("pager-page-{page}")).is_some())
      .collect()
  }

  /// The page whose box lies where the pager's does, if one does.
  fn shown(&mut self) -> Option<usize> {
    (0..self.count).find(|page| {
      self
        .tree
        .get_element_by_id_mut(&format!("pager-page-{page}"))
        .and_then(|page| page.bounds())
        .is_some_and(|rect| rect.x.abs() < 0.5 && rect.y.abs() < 0.5)
    })
  }

  fn key(&mut self, key: &str) {
    self.tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
    self.pass();
  }
}

#[test]
fn only_the_page_shown_and_the_ones_beside_it_are_built() {
  let mut stage = Stage::new(1000, false);
  assert_eq!(stage.built(), [0, 1]);
  assert_eq!(stage.shown(), Some(0));
  let made = stage.log.take();
  assert_eq!(made, ["made 0", "made 1"], "a thousand pages, and two of them made");
  let page = stage
    .tree
    .get_element_by_id_mut("pager-page-0")
    .unwrap()
    .bounds()
    .unwrap();
  assert_eq!(
    (page.width, page.height),
    (WIDTH, HEIGHT),
    "a page is as large as the pager"
  );

  // The app sets the index: the pager goes there, and the pages about it are the ones built.
  stage.index.set(500);
  stage.run(600);
  assert_eq!(stage.shown(), Some(500));
  assert_eq!(stage.built(), [499, 500, 501]);
  let told = stage.log.take();
  assert!(
    told.contains(&"dropped 0".to_owned()) && told.contains(&"made 500".to_owned()),
    "{told:?}"
  );
  assert!(
    !told.iter().any(|line| line.starts_with("index")),
    "a page the app set is not told back: {told:?}"
  );
  // No page on the way was made.
  let made: Vec<&String> = told.iter().filter(|line| line.starts_with("made")).collect();
  assert_eq!(made, ["made 499", "made 500", "made 501"]);

  // To the page beside, which is built: over a few frames.
  stage.index.set(501);
  stage.run(16);
  assert_eq!(stage.shown(), None, "it is on its way");
  stage.run(600);
  assert_eq!(stage.shown(), Some(501));
  assert_eq!(stage.built(), [500, 501, 502]);
}

#[test]
fn a_finger_turns_one_page_and_the_index_changes_when_the_page_is_at_rest() {
  let mut stage = Stage::new(5, false);
  stage.log.take();
  stage.flick((50.0, 150.0), (0.0, -60.0));
  assert_eq!(
    stage.index.get_untracked(),
    0,
    "the finger has lifted, and the page is on its way"
  );
  assert!(stage.log.take().is_empty());
  stage.run(800);
  assert_eq!(stage.index.get_untracked(), 1);
  assert_eq!(stage.shown(), Some(1));
  assert_eq!(stage.built(), [0, 1, 2]);
  assert_eq!(stage.log.take(), ["made 2", "index 1"]);

  // A hard throw goes no further than the page beside.
  stage.flick((50.0, 190.0), (0.0, -180.0));
  stage.run(800);
  assert_eq!(stage.index.get_untracked(), 2);
  assert_eq!(stage.built(), [1, 2, 3]);
  // The page after the one it goes to is made when the pages are nearer that one, before they are at rest.
  assert_eq!(stage.log.take(), ["made 3", "index 2", "dropped 0"]);

  // And back.
  stage.flick((50.0, 50.0), (0.0, 60.0));
  stage.run(800);
  assert_eq!(stage.index.get_untracked(), 1);
  assert_eq!(stage.shown(), Some(1));

  // A drag that does not go far enough comes back: the index is as it was, and nothing is told.
  stage.log.take();
  stage.finger(0, TouchPhase::Start, 50.0, 150.0);
  stage.finger(200, TouchPhase::Move, 50.0, 130.0);
  stage.finger(200, TouchPhase::Move, 50.0, 120.0);
  stage.finger(200, TouchPhase::End, 50.0, 120.0);
  stage.run(800);
  assert_eq!(stage.shown(), Some(1));
  assert!(!stage.log.take().iter().any(|line| line.starts_with("index")));
}

#[test]
fn a_page_keeps_what_is_its_own_while_it_is_built() {
  let mut stage = Stage::new(6, false);
  stage.log.take();
  // To the next page and back: the first page was beside the one shown all along, and is the same one.
  stage.flick((50.0, 150.0), (0.0, -60.0));
  stage.run(800);
  stage.flick((50.0, 50.0), (0.0, 60.0));
  stage.run(800);
  assert_eq!(stage.index.get_untracked(), 0);
  let told = stage.log.take();
  assert!(
    !told.contains(&"made 0".to_owned()) && !told.contains(&"dropped 0".to_owned()),
    "{told:?}"
  );
  // Two pages on, it is dropped, and made anew when the pager comes back.
  stage.index.set(2);
  stage.run(600);
  assert!(stage.log.take().contains(&"dropped 0".to_owned()));
  stage.index.set(0);
  stage.run(600);
  assert!(stage.log.take().contains(&"made 0".to_owned()));
}

#[test]
fn a_wheel_turns_a_page_for_a_notch_and_the_index_follows_at_rest() {
  let mut stage = Stage::new(5, false);
  stage.log.take();
  stage.tree.wheel(50.0, 100.0, 0.0, -120.0, ScrollPhase::Scroll);
  stage.pass();
  assert_eq!(stage.index.get_untracked(), 0, "the page is on its way");
  stage.run(600);
  assert_eq!(stage.index.get_untracked(), 1);
  assert_eq!(stage.shown(), Some(1), "one page, and not the 120 the wheel turned");
  assert_eq!(stage.log.take(), ["made 2", "index 1"]);

  // A touchpad's swipe is many small steps, and what it leaves rolling: one page for all of them. A wheel's
  // turns are told apart by the time between them, which is the machine's own here: a rest, and then a swipe.
  let rest = Duration::from_millis(200);
  std::thread::sleep(rest);
  for _ in 0..30 {
    stage.tree.wheel(50.0, 100.0, 0.0, -9.0, ScrollPhase::Scroll);
    stage.pass();
  }
  stage.run(600);
  assert_eq!(stage.index.get_untracked(), 2, "one swipe, one page");
  assert_eq!(stage.shown(), Some(2));

  // Back up.
  std::thread::sleep(rest);
  stage.tree.wheel(50.0, 100.0, 0.0, 120.0, ScrollPhase::Scroll);
  stage.run(600);
  assert_eq!(stage.index.get_untracked(), 1);
  // A wheel turned across is not this pager's.
  std::thread::sleep(rest);
  stage.tree.wheel(50.0, 100.0, -120.0, 0.0, ScrollPhase::Scroll);
  stage.run(600);
  assert_eq!(stage.index.get_untracked(), 1);
  assert_eq!(stage.shown(), Some(1));
}

#[test]
fn the_arrow_keys_turn_a_page_along_its_way() {
  let mut stage = Stage::new(5, false);
  stage.key("ArrowDown");
  stage.run(600);
  assert_eq!(stage.index.get_untracked(), 1);
  assert_eq!(stage.shown(), Some(1));
  stage.key("ArrowRight");
  stage.run(600);
  assert_eq!(
    stage.index.get_untracked(),
    1,
    "a key across is not a pager's that goes down"
  );
  stage.key("End");
  stage.run(600);
  assert_eq!(stage.index.get_untracked(), 4);
  stage.key("ArrowDown");
  stage.run(600);
  assert_eq!(stage.index.get_untracked(), 4, "it is at its last page");
  stage.key("PageUp");
  stage.run(600);
  assert_eq!(stage.index.get_untracked(), 3);
  stage.key("Home");
  stage.run(600);
  assert_eq!(stage.index.get_untracked(), 0);

  // While a text input has the focus the keys are the input's.
  stage.tree.get_element_by_id_mut("field").unwrap().focus();
  stage.pass();
  stage.key("ArrowDown");
  stage.run(600);
  assert_eq!(stage.index.get_untracked(), 0);

  // Across: Left and Right.
  let mut across = Stage::new(5, true);
  across.key("ArrowRight");
  across.run(600);
  assert_eq!(across.index.get_untracked(), 1);
  assert_eq!(across.shown(), Some(1));
  across.key("ArrowDown");
  across.run(600);
  assert_eq!(across.index.get_untracked(), 1);
  across.key("ArrowLeft");
  across.run(600);
  assert_eq!(across.index.get_untracked(), 0);

  // A pager told to leave the keys alone.
  let mut deaf = Stage::with(5, false, |pager| pager.keys(false));
  deaf.key("ArrowDown");
  deaf.run(600);
  assert_eq!(deaf.index.get_untracked(), 0);
}

#[test]
fn a_pager_across_takes_the_finger_that_goes_across_and_leaves_the_one_that_goes_down() {
  let mut stage = Stage::new(4, true);
  let page = stage
    .tree
    .get_element_by_id_mut("pager-page-1")
    .unwrap()
    .bounds()
    .unwrap();
  assert_eq!((page.x, page.y), (WIDTH, 0.0), "its pages lie side by side");
  stage.flick((80.0, 100.0), (-60.0, 0.0));
  stage.run(800);
  assert_eq!(stage.index.get_untracked(), 1);
  assert_eq!(stage.shown(), Some(1));
  // A finger that goes down turns nothing.
  stage.flick((50.0, 150.0), (0.0, -60.0));
  stage.run(800);
  assert_eq!(stage.index.get_untracked(), 1);
  assert_eq!(stage.shown(), Some(1));
}

#[test]
fn a_pager_without_a_signal_keeps_where_it_is_and_one_with_few_pages_stays_in_them() {
  // No signal: the pager's own index, told to the handler.
  let log = Log::default();
  let told = log.clone();
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(300, 300);
  tree.mount_root::<Scene>(
    &mut app,
    SceneProps(Arc::new(move || {
      let told = told.clone();
      Column::new()
        .size(WIDTH, HEIGHT)
        .child(
          Pager::new(3, |_: &mut Ctx, page| Column::new().id(format!("leaf-{page}")))
            .on_index_change(move |to| told.say(format!("index {to}")))
            .id("pager"),
        )
        .into()
    })),
  );
  let mut now = Instant::now();
  tree.set_animation_clock(Some(now));
  for _ in 0..3 {
    tree.pass(&mut app, &TestSurface);
  }
  tree.key_down("ArrowDown".to_owned(), "ArrowDown".to_owned(), false, false, false);
  for _ in 0..80 {
    now += Duration::from_millis(8);
    tree.set_animation_clock(Some(now));
    tree.pass(&mut app, &TestSurface);
  }
  assert_eq!(log.take(), ["index 1"]);
  assert!(
    tree.get_element_by_id("leaf-2").is_some(),
    "the page beside the one shown is built"
  );

  // One page: nothing to turn, and an index past the end is the last page.
  let mut stage = Stage::new(1, false);
  stage.index.set(7);
  stage.run(200);
  assert_eq!(stage.built(), [0]);
  assert_eq!(stage.shown(), Some(0));
  // No pages at all: an empty box.
  let mut stage = Stage::new(0, false);
  assert!(stage.built().is_empty());
}

#[test]
fn a_strip_across_in_a_feed_that_goes_down_is_turned_by_itself() {
  // A feed of three albums, each a strip of three pictures; the app holds where each is.
  let album = Signal::new(0);
  let pictures: Arc<Vec<Signal<usize>>> = Arc::new((0..3).map(|_| Signal::new(0)).collect());
  let scene = {
    let (album, pictures) = (album.clone(), pictures.clone());
    Arc::new(move || -> Element {
      let pictures = pictures.clone();
      let feed = Pager::new(3, move |ctx: &mut Ctx, album| {
        Pager::new(3, move |_: &mut Ctx, picture| {
          Column::new().id(format!("tile-{album}-{picture}"))
        })
        .horizontal()
        .index(pictures[album].clone())
        .id(format!("strip-{album}"))
        .mount(ctx)
      })
      .index(album.clone())
      .id("pager");
      Column::new().size(WIDTH, HEIGHT).child(feed).into()
    })
  };
  let mut stage = Stage::of(3, album.clone(), Log::default(), scene);
  let at = |pictures: &Arc<Vec<Signal<usize>>>| -> Vec<usize> { pictures.iter().map(Signal::get_untracked).collect() };
  assert_eq!(stage.built(), [0, 1]);
  assert!(
    stage.tree.get_element_by_id("tile-0-1").is_some(),
    "the picture beside the first is built"
  );
  assert!(
    stage.tree.get_element_by_id("tile-1-0").is_some(),
    "and the next album's first"
  );
  assert!(stage.tree.get_element_by_id("tile-2-0").is_none());

  // Across: the strip of the album shown, and not the feed.
  stage.flick((80.0, 100.0), (-60.0, 0.0));
  stage.run(800);
  assert_eq!((album.get_untracked(), at(&pictures)), (0, vec![1, 0, 0]));
  // Down: the feed, and no strip.
  stage.flick((50.0, 150.0), (0.0, -60.0));
  stage.run(800);
  assert_eq!((album.get_untracked(), at(&pictures)), (1, vec![1, 0, 0]));
  assert_eq!(stage.shown(), Some(1));

  // The keys across are the strip's that shows, and not the one of the album that went by, which is still
  // built; the keys down are the feed's.
  stage.key("ArrowRight");
  stage.run(600);
  assert_eq!((album.get_untracked(), at(&pictures)), (1, vec![1, 1, 0]));
  stage.key("ArrowDown");
  stage.run(600);
  assert_eq!((album.get_untracked(), at(&pictures)), (2, vec![1, 1, 0]));

  // Back at the first album, its strip is where the app's signal says: it was dropped, and made anew.
  album.set(0);
  stage.run(600);
  assert_eq!(stage.shown(), Some(0));
  let tile = stage.tree.get_element_by_id_mut("tile-0-1").unwrap().bounds().unwrap();
  assert_eq!((tile.x, tile.y), (0.0, 0.0), "the second picture is the one that shows");
}
