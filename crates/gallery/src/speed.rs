//! How long the gallery's longest pages take to build: run by hand in release.
//!
//! `cargo test -p gallery --release -- speed --ignored --nocapture`

use std::time::{Duration, Instant};

use lurq::app::{App, Tree};

use crate::{Gallery, GalleryProps};

/// The first build and layout of `route`, then the root built again and laid out.
fn measure(route: &str) -> (Duration, Duration, usize) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(1440, 900);
  let started = Instant::now();
  tree.mount_root::<Gallery>(&mut app, GalleryProps { route: route.to_owned() });
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
