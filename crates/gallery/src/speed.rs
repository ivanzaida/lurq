//! How long the gallery's longest pages take to build: run by hand in release.
//!
//! `cargo test -p gallery --release -- speed --ignored --nocapture`

use std::time::{Duration, Instant};

use lurq::app::{App, Tree, events::MouseButton};

use crate::{Gallery, GalleryProps};

/// The first build and layout of `route`, then the root built again and laid out.
fn measure(route: &str) -> (Duration, Duration, usize) {
  let mut app = App::new();
  let mut tree = Tree::new();
  crate::pages::engine::prepare(&mut tree);
  tree.resize(1440, 900);
  let started = Instant::now();
  tree.mount_root::<Gallery>(
    &mut app,
    GalleryProps {
      route: route.to_owned(),
    },
  );
  tree.pass_headless(&mut app);
  let first = started.elapsed();
  let started = Instant::now();
  tree.rebuild();
  tree.pass_headless(&mut app);
  let again = started.elapsed();
  let nodes = std::cell::Cell::new(0);
  tree.find_element(|_| {
    nodes.set(nodes.get() + 1);
    false
  });
  (first, again, nodes.get())
}

#[test]
#[ignore = "a measure, run by hand in release"]
fn speed_of_the_longest_pages() {
  for route in ["/data-grid", "/table", "/markdown"] {
    // The least of five runs: what the work costs, without what else the machine does.
    let runs: Vec<_> = (0..5).map(|_| measure(route)).collect();
    let first = runs.iter().map(|run| run.0).min().unwrap_or_default();
    let again = runs.iter().map(|run| run.1).min().unwrap_or_default();
    println!(
      "{route} ({} nodes): first build and layout {:.1} ms, built again {:.1} ms",
      runs[0].2,
      first.as_secs_f64() * 1000.0,
      again.as_secs_f64() * 1000.0
    );
  }
}

/// A row of the Data Grid page selected, again and again: a press of one of its cells, then Space, which selects
/// and unselects its row. The median frame of the Space presses.
fn select_row() -> Option<Duration> {
  let mut app = App::new();
  let mut tree = Tree::new();
  crate::pages::engine::prepare(&mut tree);
  tree.resize(1440, 900);
  tree.mount_root::<Gallery>(
    &mut app,
    GalleryProps {
      route: "/data-grid".to_owned(),
    },
  );
  tree.pass_headless(&mut app);
  tree.pass_headless(&mut app);
  // The first grid's cell of its third row, second column.
  let id = std::cell::RefCell::new(None);
  tree.find_element(|element| {
    if let Some(found) = element.id().filter(|found| found.ends_with("-cell-2-1"))
      && id.borrow().is_none()
    {
      *id.borrow_mut() = Some(found.to_owned());
    }
    false
  });
  let id = id.into_inner()?;
  let (x, y) = tree.get_element_by_id_mut(&id)?.bounds()?.center();
  tree.mouse_down(x, y, MouseButton::Left);
  tree.mouse_up(x, y, MouseButton::Left);
  tree.pass_headless(&mut app);
  let mut frames = Vec::new();
  for _ in 0..10 {
    let started = Instant::now();
    tree.key_down(" ".into(), "Space".into(), false, false, false);
    tree.key_up(" ".into(), "Space".into(), false, false, false);
    tree.pass_headless(&mut app);
    frames.push(started.elapsed());
  }
  frames.sort();
  Some(frames[frames.len() / 2])
}

#[test]
#[ignore = "a measure, run by hand in release"]
fn speed_of_a_row_selected_on_the_data_grid_page() {
  let runs: Vec<_> = (0..5).filter_map(|_| select_row()).collect();
  let least = runs.iter().min().copied().unwrap_or_default();
  println!(
    "/data-grid, a row selected: {:.2} ms a frame",
    least.as_secs_f64() * 1000.0
  );
}
