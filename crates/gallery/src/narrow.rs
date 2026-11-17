//! The gallery in a window as narrow as a phone, laid out with no window: what the shell and a page show stays
//! in the box that holds it. `scripts/gallery-narrow.py` walks the running gallery for the same and for more
//! (what lies over what, what is squeezed), with pictures.
//!
//! A page that fails here has something that does not get narrower with the window: a row that should wrap,
//! a scene of a fixed width that should be a greatest width or scroll sideways in its card (`kit::fit`), a
//! text beside others on one line. `NARROW_PAGES=/button,/table cargo test -p gallery narrow -- --nocapture`
//! lays out those pages alone.

use std::{
  collections::{HashMap, HashSet},
  time::{Duration, Instant},
};

use lurq::{
  app::{App, Tree},
  layout::layout_result::LayoutResult,
  node::ElementRef,
};

use crate::{Gallery, GalleryProps, pages::PAGES};

/// How far past its box something may be laid out: a fraction of a pixel is rounding.
const TOLERANCE: f32 = 1.5;
/// How high the window is: a phone's.
const HEIGHT: u32 = 844;
/// A desktop's width. What reaches past its box there does so on purpose, at every width: a badge on a corner,
/// a thumb at the end of its track, a ring.
const WIDE: u32 = 1440;

/// An element as it was laid out: what it is, what it says, and its box in the window.
struct Laid {
  tag: String,
  id: String,
  /// The first words in it, to tell it from its like.
  said: String,
  x: f32,
  y: f32,
  width: f32,
  height: f32,
  kids: Vec<Laid>,
  /// What a window of another width calls the same element: see [`name`].
  key: String,
}

fn laid(element: ElementRef<'_>, layout: &LayoutResult, x: f32, y: f32) -> Laid {
  let dropped = layout.is_dropped();
  let kids: Vec<Laid> = element
    .children()
    .into_iter()
    .zip(&layout.children)
    .map(|(child, placed)| laid(child, &placed.result, x + placed.offset.x, y + placed.offset.y))
    .collect();
  let own = element.text_content().unwrap_or_default();
  let said = if own.is_empty() {
    kids
      .iter()
      .map(|kid| kid.said.as_str())
      .find(|said| !said.is_empty())
      .unwrap_or_default()
      .to_owned()
  } else {
    own.chars().take(40).collect()
  };
  Laid {
    tag: element.tag_name().to_owned(),
    id: element.id().unwrap_or_default().to_owned(),
    said,
    x,
    y,
    width: if dropped { 0.0 } else { layout.size.width },
    height: if dropped { 0.0 } else { layout.size.height },
    kids,
    key: String::new(),
  }
}

/// The gallery on `route` in a window `width` wide, laid out until it is settled, and once more a moment later:
/// what moves for ever (the bar of a progress with no value) is somewhere else then.
fn gallery(route: &str, width: u32) -> [Laid; 2] {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(width, HEIGHT);
  crate::pages::engine::prepare(&mut tree);
  tree.mount_root::<Gallery>(
    &mut app,
    GalleryProps {
      route: route.to_owned(),
    },
  );
  // A page measures the room it has and lays itself out in it: a few passes, and it is settled. A second goes
  // by between them, so that what comes in with a motion (a bar that slides up) has come in.
  let mut now = Instant::now();
  for _ in 0..8 {
    tree.set_animation_clock(Some(now));
    tree.pass_headless(&mut app);
    now += Duration::from_secs(1);
  }
  let mut once = |tree: &mut Tree, now: Instant| {
    tree.set_animation_clock(Some(now));
    tree.pass_headless(&mut app);
    let root = tree.root().expect("the gallery is mounted");
    let layout = tree.last_layout().expect("the gallery is laid out");
    let mut root = laid(root, layout, 0.0, 0.0);
    name(&mut root);
    root
  };
  let first = once(&mut tree, now);
  // What runs on the process's own clock takes a moment of it to move.
  std::thread::sleep(Duration::from_millis(40));
  let second = once(&mut tree, now + Duration::from_secs(1));
  [first, second]
}

/// The column of the page: its head and its sections, which a window of any width has in the same order.
fn page(node: &mut Laid) -> Option<&mut Laid> {
  if node.tag == "ScrollVertical" && node.kids.first().is_some_and(|kid| kid.tag == "Outlet") {
    let mut node = node;
    loop {
      // In a wide window the list of the page's sections stands beside the column.
      let beside = node.kids.len() == 2 && node.kids[1].said.to_uppercase().starts_with("ON THIS PAGE");
      if node.kids.len() != 1 && !beside {
        return Some(node);
      }
      node = &mut node.kids[0];
    }
  }
  node.kids.iter_mut().find_map(page)
}

/// Gives each element a key that a window of another width gives the same element: the part of the page it is
/// in, what it is and what it says, and which of those that say the same it is.
fn name(root: &mut Laid) {
  fn number(node: &mut Laid, part: &str, seen: &mut HashMap<String, usize>) {
    let base = format!("{part}|{}|{}|{}", node.tag, node.id, node.said);
    let count = seen.entry(base.clone()).or_insert(0);
    *count += 1;
    node.key = format!("{base}#{count}");
    for kid in &mut node.kids {
      number(kid, part, seen);
    }
  }
  let mut seen = HashMap::new();
  if let Some(page) = page(root) {
    for (index, section) in page.kids.iter_mut().enumerate() {
      number(section, &index.to_string(), &mut seen);
    }
  }
  // What is left is the shell, and the boxes that hold the page.
  fn rest(node: &mut Laid, seen: &mut HashMap<String, usize>) {
    if !node.key.is_empty() {
      return;
    }
    let base = format!("shell|{}|{}|{}", node.tag, node.id, node.said);
    let count = seen.entry(base.clone()).or_insert(0);
    *count += 1;
    node.key = format!("{base}#{count}");
    for kid in &mut node.kids {
      rest(kid, seen);
    }
  }
  rest(root, &mut seen);
}

/// An edge of the box nearest around an element, and what the box is, to say it.
#[derive(Clone)]
struct Edge {
  at: f32,
  of: String,
}

/// The four edges an element is held in.
#[derive(Clone)]
struct Held {
  left: Edge,
  right: Edge,
  top: Edge,
  bottom: Edge,
}

/// What runs past its box: which way, the element's key, and a line that says it.
struct Found {
  kind: &'static str,
  key: String,
  /// What it is and what it is past, with no words: what tells the same fault where a window of another width
  /// has other words in it (a chart names other weeks on a shorter axis).
  shape: String,
  /// Where its box is, to tell what stands still from what is moving.
  at: (f32, f32, f32, f32),
  line: String,
}

/// The shape of a fault of `node` against what holds it.
fn shape(node: &Laid, holder: &str) -> String {
  let part = node.key.split('|').next().unwrap_or_default();
  let holder = holder.split(' ').next().unwrap_or_default();
  format!("{part}|{}|{}|{holder}", node.tag, node.id)
}

fn describe(node: &Laid) -> String {
  let mut what = node.tag.clone();
  if !node.id.is_empty() {
    what.push_str(&format!(" #{}", node.id));
  }
  if !node.said.is_empty() {
    what.push_str(&format!(" {:?}", node.said));
  }
  what
}

fn measure(node: &Laid, held: &Held, found: &mut Vec<Found>) {
  // What has no size shows nothing of what is in it.
  if node.width <= 0.0 || node.height <= 0.0 {
    return;
  }
  let what = describe(node);
  let mut held = held.clone();
  let edge = |at: f32| Edge { at, of: what.clone() };
  let (right, bottom) = (node.x + node.width, node.y + node.height);
  let over = (right - held.right.at).max(held.left.at - node.x);
  if over > TOLERANCE {
    found.push(Found {
      kind: "past",
      key: node.key.clone(),
      shape: shape(node, &held.right.of),
      at: (node.x, node.y, node.width, node.height),
      line: format!(
        "{what} at {:.0},{:.0}, {:.0} wide, is {over:.0} past {} ({:.0} to {:.0})",
        node.x, node.y, node.width, held.right.of, held.left.at, held.right.at
      ),
    });
    // What is in it is held by it from here on: one fault is said once.
    held.left = edge(node.x);
    held.right = edge(right);
  } else {
    if node.x > held.left.at {
      held.left = edge(node.x);
    }
    if right < held.right.at {
      held.right = edge(right);
    }
  }
  let over = (bottom - held.bottom.at).max(held.top.at - node.y);
  if over > TOLERANCE {
    found.push(Found {
      kind: "under",
      key: node.key.clone(),
      shape: shape(node, &held.bottom.of),
      at: (node.x, node.y, node.width, node.height),
      line: format!(
        "{what} at {:.0},{:.0}, {:.0} high, is {over:.0} out of {} ({:.0} to {:.0})",
        node.x, node.y, node.height, held.bottom.of, held.top.at, held.bottom.at
      ),
    });
    held.top = edge(node.y);
    held.bottom = edge(bottom);
  } else {
    if node.y > held.top.at {
      held.top = edge(node.y);
    }
    if bottom < held.bottom.at {
      held.bottom = edge(bottom);
    }
  }
  // What scrolls holds what is larger than it, the way it scrolls. A component whose root is a scroll container
  // gives that root its own tag: the engine's that do are named with the containers.
  if matches!(node.tag.as_str(), "ScrollHorizontal" | "ScrollBoth" | "ZoomScroll") {
    held.left.at = f32::NEG_INFINITY;
    held.right.at = f32::INFINITY;
  }
  if matches!(
    node.tag.as_str(),
    "ScrollVertical" | "ScrollBoth" | "ZoomScroll" | "VirtualizedList"
  ) {
    held.top.at = f32::NEG_INFINITY;
    held.bottom.at = f32::INFINITY;
  }
  for kid in &node.kids {
    measure(kid, &held, found);
  }
}

/// What runs past its box on `route` in a window `width` wide, and stands still there: what is past its box at
/// one moment and elsewhere the next is moving through it, as a bar that says "working" does.
fn past(route: &str, width: u32) -> Vec<Found> {
  let window = |at: f32| Edge {
    at,
    of: "the window".to_owned(),
  };
  let held = Held {
    left: window(0.0),
    right: window(width as f32),
    top: window(0.0),
    bottom: window(HEIGHT as f32),
  };
  let [first, second] = gallery(route, width).map(|root| {
    let mut found = Vec::new();
    measure(&root, &held, &mut found);
    found
  });
  let still = |a: &Found, b: &Found| {
    a.kind == b.kind
      && a.key == b.key
      && (a.at.0 - b.at.0).abs() < 0.5
      && (a.at.1 - b.at.1).abs() < 0.5
      && (a.at.2 - b.at.2).abs() < 0.5
      && (a.at.3 - b.at.3).abs() < 0.5
  };
  first
    .into_iter()
    .filter(|found| second.iter().any(|later| still(found, later)))
    .collect()
}

/// The pages `NARROW_PAGES` names, or all of them.
fn only() -> Option<Vec<String>> {
  std::env::var("NARROW_PAGES")
    .ok()
    .map(|pages| pages.split(',').map(str::to_owned).collect())
}

/// What runs past its box at 390 and 320 on each page `chosen` keeps, and not at a desktop's width.
fn narrow_faults(chosen: impl Fn(&str) -> bool) -> Vec<String> {
  let mut lines = Vec::new();
  for page in PAGES {
    if !chosen(page.path) {
      continue;
    }
    let wide = past(page.path, WIDE);
    let known: HashSet<(&str, &str)> = wide.iter().map(|found| (found.kind, found.key.as_str())).collect();
    for width in [390, 320] {
      let narrow = past(page.path, width);
      let still: HashSet<(&str, &str)> = narrow.iter().map(|found| (found.kind, found.key.as_str())).collect();
      // What the wide window has under a name the narrow one has not: the same fault with other words in it,
      // as many times as it is there.
      let mut renamed: HashMap<(&str, &str), usize> = HashMap::new();
      for found in &wide {
        if !still.contains(&(found.kind, found.key.as_str())) {
          *renamed.entry((found.kind, found.shape.as_str())).or_insert(0) += 1;
        }
      }
      for found in &narrow {
        if known.contains(&(found.kind, found.key.as_str())) {
          continue;
        }
        if let Some(left) = renamed.get_mut(&(found.kind, found.shape.as_str()))
          && *left > 0
        {
          *left -= 1;
          continue;
        }
        lines.push(format!("{} at {width}: {} {}", page.path, found.kind, found.line));
      }
    }
  }
  lines
}

/// Every page, at a phone's width and at the least the window has. One test, one page after another: a page
/// leaves the width of its card for the next to start with, which two at once would take from each other.
#[test]
fn nothing_runs_past_its_box_in_a_narrow_window() {
  let only = only();
  let lines = narrow_faults(|path| only.as_ref().is_none_or(|only| only.iter().any(|named| named == path)));
  assert!(
    lines.is_empty(),
    "{} run past their boxes in a narrow window:\n{}",
    lines.len(),
    lines.join("\n")
  );
}

/// The styling section of a playground, open in place of the props, at a phone's width and at the least the window
/// has: pages whose components declare many parts and states, and a page with nothing to style. `NARROW_PAGES`
/// names others.
#[test]
fn the_styling_section_holds_in_a_narrow_window() {
  const STYLED: [&str; 7] = [
    "/button",
    "/switch",
    "/tabs",
    "/table",
    "/date-picker",
    "/sidebar",
    "/engine/layout",
  ];
  crate::playground::open_styling_first(true);
  let only = only();
  let lines = narrow_faults(|path| match &only {
    Some(only) => only.iter().any(|named| named == path),
    None => STYLED.contains(&path),
  });
  crate::playground::open_styling_first(false);
  assert!(
    lines.is_empty(),
    "{} run past their boxes in the styling section:\n{}",
    lines.len(),
    lines.join("\n")
  );
}

/// The tabs under a playground's stage are whole in their bar at every width: the line under the selected one is
/// at their foot, and a bar lower than the tabs cut it off.
#[test]
fn the_tabs_under_a_stage_are_whole_in_their_bar() {
  for width in [WIDE, 390] {
    let cut: Vec<String> = past("/button", width)
      .into_iter()
      .filter(|found| found.kind == "under" && found.line.contains("\"Code\""))
      .map(|found| found.line)
      .collect();
    assert!(
      cut.is_empty(),
      "at {width}: {}",
      cut.join(
        "
"
      )
    );
  }
}
