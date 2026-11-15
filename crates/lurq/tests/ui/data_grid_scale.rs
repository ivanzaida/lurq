//! What a grid of a million rows by two hundred columns costs: measured, not asserted. Run with
//! `cargo test --release -p lurq --features form,router,mcp,canvas --test ui_tests data_grid_scale -- --ignored
//! --nocapture`.

use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use lurq::{
  app::events::{MouseButton, ScrollPhase},
  components::Column,
  layout::{Constraints, Size},
  node::ElementRef,
  ui::{DataGrid, TableColumn},
};

use super::stage::Stage;

#[derive(Clone)]
struct Row {
  index: u32,
}

fn working_set() -> u64 {
  let pid = std::process::id();
  let out = std::process::Command::new("powershell")
    .args([
      "-NoProfile",
      "-Command",
      &format!("(Get-Process -Id {pid}).WorkingSet64"),
    ])
    .output()
    .unwrap();
  String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0)
}

fn count(element: ElementRef<'_>) -> usize {
  1 + element.children().into_iter().map(count).sum::<usize>()
}

fn nodes(stage: &Stage) -> usize {
  stage.tree.root().map(count).unwrap_or(0)
}

fn rows_built(stage: &Stage) -> usize {
  fn walk(element: ElementRef<'_>, n: &mut usize) {
    if element.id().is_some_and(|id| id.starts_with("g-row-")) {
      *n += 1;
    }
    for child in element.children() {
      walk(child, n);
    }
  }
  let mut n = 0;
  if let Some(root) = stage.tree.root() {
    walk(root, &mut n);
  }
  n
}

fn pass(stage: &mut Stage) -> Duration {
  let start = Instant::now();
  stage.pass_after(Duration::from_millis(16));
  start.elapsed()
}

#[test]
#[ignore]
fn data_grid_scale() {
  let rows_wanted: u32 = std::env::var("ROWS")
    .ok()
    .and_then(|n| n.parse().ok())
    .unwrap_or(1_000_000);
  const COLUMNS: usize = 200;
  let base = working_set();
  let made = Instant::now();
  let rows: Arc<Vec<Row>> = Arc::new((0..rows_wanted).map(|index| Row { index }).collect());
  let columns: Arc<Vec<TableColumn<Row>>> = Arc::new(
    (0..COLUMNS)
      .map(|column| {
        TableColumn::new(&format!("c{column}"), &format!("Column {column}"))
          .width(120.0)
          .text(move |row: &Row| format!("{}·{column}", row.index))
      })
      .collect(),
  );
  println!("data: {rows_wanted} rows made in {:?}", made.elapsed());
  let start = Instant::now();
  let empty = Stage::new(|_| Column::new().size(10.0, 10.0).into());
  println!("an empty stage, for the baseline: {:?}", start.elapsed());
  drop(empty);
  let small = Instant::now();
  let ten: Arc<Vec<Row>> = Arc::new((0..10).map(|index| Row { index }).collect());
  let few = columns.clone();
  let mut stage_small = Stage::new(move |ctx| {
    Column::new()
      .size(1200.0, 800.0)
      .child(
        DataGrid::new_shared((*few).clone(), ten.clone())
          .id("g")
          .height(760.0)
          .mount(ctx),
      )
      .into()
  });
  stage_small.pass();
  println!("a grid of 10 rows by 200 columns: {:?}", small.elapsed());
  drop(stage_small);
  let with_data = working_set();
  let start = Instant::now();
  let selected: lurq::core::Signal<Vec<usize>> = lurq::core::Signal::new(Vec::new());
  let chosen = selected.clone();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(1200.0, 800.0)
      .child(
        DataGrid::new_shared((*columns).clone(), rows.clone())
          .id("g")
          .label("Big")
          .height(760.0)
          .selected(chosen.clone())
          .mount(ctx),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(1280.0, 900.0))));
  stage.pass();
  let built = start.elapsed();
  println!(
    "first show: {built:?}; nodes {}; rows built {}; memory +{} MB over the data (data +{} MB)",
    nodes(&stage),
    rows_built(&stage),
    (working_set().saturating_sub(with_data)) / 1_000_000,
    (with_data.saturating_sub(base)) / 1_000_000,
  );
  let idle = pass(&mut stage);
  println!("a pass that changes nothing: {idle:?}");
  selected.set(vec![3]);
  let render = pass(&mut stage);
  println!("a render of the grid (a row selected, the key index made): {render:?}");
  selected.set(vec![3, 4]);
  let render = pass(&mut stage);
  println!("a render of the grid (another row selected): {render:?}");

  // The wheel: twenty notches down.
  let mut times = Vec::new();
  for _ in 0..20 {
    stage.tree.scroll(600.0, 400.0, 0.0, -120.0, ScrollPhase::Scroll);
    times.push(pass(&mut stage));
  }
  times.sort();
  println!(
    "wheel: median frame {:?}, worst {:?}; rows built {}; nodes {}",
    times[times.len() / 2],
    times[times.len() - 1],
    rows_built(&stage),
    nodes(&stage)
  );

  // The bar's thumb: dragged from the top of the bar to its middle.
  let (x, y, width, height) = stage.bounds("g");
  let bar_x = x + width - 4.0;
  let start = Instant::now();
  stage.tree.mouse_move(bar_x, y + 50.0);
  stage.tree.mouse_down(bar_x, y + 50.0, MouseButton::Left);
  let mut drag = Vec::new();
  for step in 1..=10 {
    let to = y + 50.0 + (height / 2.0 - 50.0) * step as f32 / 10.0;
    stage.tree.mouse_move(bar_x, to);
    drag.push(pass(&mut stage));
  }
  stage.tree.mouse_up(bar_x, y + height / 2.0, MouseButton::Left);
  pass(&mut stage);
  drag.sort();
  println!(
    "thumb drag: median frame {:?}, worst {:?}, whole {:?}; rows built {}; first row built {:?}",
    drag[drag.len() / 2],
    drag[drag.len() - 1],
    start.elapsed(),
    rows_built(&stage),
    first_row(&stage)
  );

  // Ctrl+End from the first cell.
  let first = first_row(&stage).unwrap();
  stage.click(&format!("g-cell-{}-0", first + 12));
  println!(
    "focused after the click: {:?}",
    stage.tree.focused_element().and_then(|e| e.id().map(str::to_owned))
  );
  let start = Instant::now();
  stage.tree.key_down("End".into(), "End".into(), false, true, false);
  stage.tree.key_up("End".into(), "End".into(), false, true, false);
  let frame = pass(&mut stage);
  pass(&mut stage);
  println!(
    "Ctrl+End: frame {frame:?}, whole {:?}; focused {:?}; first row built {:?}",
    start.elapsed(),
    stage.tree.focused_element().and_then(|e| e.id().map(str::to_owned)),
    first_row(&stage)
  );
  let last = rows_wanted as usize;
  let tops: Vec<f32> = (last - 10..last)
    .map(|row| stage.bounds(&format!("g-row-{row}")).1)
    .collect();
  println!("the last rows' tops: {tops:?}");
  let start = Instant::now();
  stage.tree.key_down("a".into(), "a".into(), false, true, false);
  stage.tree.key_up("a".into(), "a".into(), false, true, false);
  let all = pass(&mut stage);
  println!(
    "Ctrl+A, every row selected: frame {all:?}, whole {:?}, selected {}",
    start.elapsed(),
    selected.get().len()
  );
  selected.set(Vec::new());
  pass(&mut stage);
  println!(
    "memory at the end: +{} MB over the data",
    (working_set().saturating_sub(with_data)) / 1_000_000
  );
}

fn first_row(stage: &Stage) -> Option<usize> {
  fn walk(element: ElementRef<'_>, out: &mut Vec<usize>) {
    if let Some(n) = element
      .id()
      .and_then(|id| id.strip_prefix("g-row-"))
      .and_then(|n| n.parse().ok())
    {
      out.push(n);
    }
    for child in element.children() {
      walk(child, out);
    }
  }
  let mut out = Vec::new();
  if let Some(root) = stage.tree.root() {
    walk(root, &mut out);
  }
  out.into_iter().min()
}
