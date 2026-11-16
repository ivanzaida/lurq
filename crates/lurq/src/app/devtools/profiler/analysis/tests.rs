use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

use super::{
  super::{
    calls,
    flame::{CHART_ID, SELECTION_ID, TOOLTIP_ID, VIEW_ID, color_of, track_label},
  },
  *,
};
use crate::app::{
  App, Tree,
  events::{MouseButton, ScrollPhase},
  profiler::{
    ApplicationLane, ApplicationScopeId, ApplicationScopeInfo, ApplicationScopeSample, PassSample, Phase, PhaseSpan,
    ProfileSample, SampleData,
  },
};

/// A pass of `duration_ms` at `start_ms` with the phases of a real one:
/// layout_update (ui_rebuild with ui_commit, layout_compute,
/// component_after_layout) then render (acquire, encode holding `commands`
/// canvas_commands, submit, present), each at a fixed part of the pass.
pub(crate) fn layered_pass(sequence: u64, start_ms: f64, duration_ms: f64, commands: usize) -> Arc<ProfileSample> {
  let start = (start_ms * 1e6) as u64;
  let duration = (duration_ms * 1e6) as u64;
  let at = |part: f64| start + (duration as f64 * part) as u64;
  let span = |phase, depth, from: f64, to: f64| PhaseSpan {
    phase,
    depth,
    start_ns: at(from),
    end_ns: at(to),
    component: None,
  };
  let mut spans = vec![
    span(Phase::PassSetup, 0, 0.0, 1.0),
    span(Phase::LayoutUpdate, 1, 0.02, 0.6),
    span(Phase::Rebuild, 2, 0.04, 0.3),
    span(Phase::Commit, 3, 0.2, 0.3),
    span(Phase::LayoutCompute, 2, 0.32, 0.5),
    span(Phase::ComponentAfterLayout, 2, 0.52, 0.58),
    span(Phase::Render, 1, 0.62, 0.98),
    span(Phase::RenderAcquire, 2, 0.63, 0.66),
    span(Phase::RenderEncode, 2, 0.66, 0.9),
    span(Phase::RenderSubmit, 2, 0.9, 0.94),
    span(Phase::RenderPresent, 2, 0.94, 0.97),
  ];
  let step = 0.22 / commands.max(1) as f64;
  for command in 0..commands {
    let from = 0.67 + step * command as f64;
    spans.push(span(Phase::CanvasCommands, 3, from, from + step * 0.8));
  }
  spans.sort_by_key(|span| (span.start_ns, span.depth));
  Arc::new(ProfileSample {
    sequence,
    window: "main".into(),
    started_ms: start_ms,
    completed_ms: start_ms + duration_ms,
    data: SampleData::Pass(PassSample {
      frame_id: Some(sequence),
      rendered: true,
      cached_render_list: false,
      total: Duration::from_nanos(duration),
      layout_update: Duration::ZERO,
      layout_compute: Duration::ZERO,
      component_after_layout: Duration::ZERO,
      layout_recalculated: true,
      canvas_recording: Duration::ZERO,
      canvas_preparation: Duration::ZERO,
      canvas_text: None,
      backend: "none",
      frame: None,
      spans,
      dropped_spans: 0,
      reasons: Default::default(),
      present: None,
      counters: None,
      screenshot: false,
      gpu: None,
      component_layouts: Vec::new(),
    }),
  })
}

pub(crate) fn scope(
  id: u64,
  parent: Option<u64>,
  label: &'static str,
  lane: ApplicationLane,
  start_ms: f64,
  wall_ms: f64,
) -> Arc<ApplicationScopeSample> {
  Arc::new(ApplicationScopeSample {
    scope: ApplicationScopeInfo {
      id: ApplicationScopeId(id),
      parent_id: parent.map(ApplicationScopeId),
      depth: u32::from(parent.is_some()),
      window: "main".into(),
      label,
      lane,
      started_ms: start_ms,
    },
    completed_ms: start_ms + wall_ms,
    wall: Duration::from_secs_f64(wall_ms / 1000.0),
  })
}

pub(crate) fn capture_of(
  passes: Vec<Arc<ProfileSample>>,
  scopes: Vec<Arc<ApplicationScopeSample>>,
  ended_ms: f64,
) -> ProfilerCapture {
  ProfilerCapture {
    started_ms: 0.0,
    ended_ms,
    passes,
    markers: Vec::new(),
    scopes,
    dropped_scopes: 0,
    screenshots: Vec::new(),
    size_bytes: 0,
    stopped_at_limit: false,
    boundary_excluded: 0,
    gpu_timestamps: Default::default(),
  }
}

/// `passes` passes at 60 Hz of 2 to 8 ms, each with `commands` canvas
/// commands, and application scopes on both lanes: a worker `load` holding
/// a `decode` and a UI `click` every 20 passes, the last 128 kept.
pub(crate) fn synthetic_capture(passes: usize, commands: usize) -> ProfilerCapture {
  let frame_ms = 1000.0 / 60.0;
  let samples = (0..passes)
    .map(|index| {
      layered_pass(
        index as u64 + 1,
        10.0 + index as f64 * frame_ms,
        2.0 + (index % 7) as f64,
        commands,
      )
    })
    .collect();
  let mut scopes = Vec::new();
  for (index, pass) in (0..passes).step_by(20).enumerate() {
    let at = 10.0 + pass as f64 * frame_ms;
    let id = index as u64 * 3 + 1;
    scopes.push(scope(id, None, "load", ApplicationLane::Worker, at, 30.0));
    scopes.push(scope(
      id + 1,
      Some(id),
      "decode",
      ApplicationLane::Worker,
      at + 5.0,
      20.0,
    ));
    scopes.push(scope(id + 2, None, "click", ApplicationLane::Ui, at + 1.0, 1.0));
  }
  let kept = scopes.len().saturating_sub(MAX_APPLICATION_SCOPES);
  let scopes = scopes.split_off(kept);
  capture_of(samples, scopes, 20.0 + passes as f64 * frame_ms)
}

const MAX_APPLICATION_SCOPES: usize = crate::app::profiler::MAX_APPLICATION_SCOPES_PER_SESSION;

fn three_passes() -> Arc<ProfilerCapture> {
  Arc::new(capture_of(
    vec![
      layered_pass(1, 10.0, 10.0, 0),
      layered_pass(2, 40.0, 10.0, 0),
      layered_pass(3, 70.0, 10.0, 0),
    ],
    vec![scope(1, None, "load", ApplicationLane::Worker, 42.0, 5.0)],
    100.0,
  ))
}

fn whole(capture: &ProfilerCapture) -> TimeRange {
  capture.whole()
}

fn name_of(analysis: &Analysis, block: &Block) -> Option<&'static str> {
  block.span.map(|span| analysis.span(span).name)
}

fn close(a: f32, b: f32, what: &str) {
  assert!((a - b).abs() < 0.5, "{what}: {a} vs {b}");
}

#[test]
fn nested_spans_are_drawn_to_scale_for_one_frame() {
  let capture = three_passes();
  let analysis = Analysis::new(&capture, AnalysisScope::Frame, whole(&capture), Some(2));
  assert_eq!(analysis.passes, 1);
  assert_eq!(
    analysis.basis,
    TimeRange {
      start_ms: 40.0,
      end_ms: 50.0
    }
  );
  let frames = &analysis.tracks[0];
  assert_eq!(frames.track, SpanTrack::Frames);
  assert_eq!(frames.rows.len(), 4);
  let blocks = |depth: usize| row_blocks(&analysis.forest, &frames.rows[depth], analysis.basis, 1000.0);
  let root = blocks(0);
  assert_eq!(root.len(), 1);
  close(root[0].x, 0.0, "pass start");
  close(root[0].width, 1000.0, "pass width");
  let phases = blocks(1);
  assert_eq!(
    phases.iter().map(|block| name_of(&analysis, block)).collect::<Vec<_>>(),
    [Some("layout_update"), Some("render")]
  );
  close(phases[0].x, 20.0, "layout_update start");
  close(phases[0].width, 580.0, "layout_update width");
  close(phases[1].x, 620.0, "render start");
  let commit = blocks(3);
  assert_eq!(name_of(&analysis, &commit[0]), Some("ui_commit"));
  close(commit[0].x, 200.0, "ui_commit start");
  close(commit[0].width, 100.0, "ui_commit width");
  // The application scope that overlaps the pass is on its lane's track.
  assert_eq!(
    analysis.tracks.iter().map(|track| track.track).collect::<Vec<_>>(),
    [SpanTrack::Frames, SpanTrack::Application(ApplicationLane::Worker)]
  );
}

#[test]
fn a_range_draws_every_pass_at_its_time() {
  let capture = three_passes();
  let analysis = Analysis::new(&capture, AnalysisScope::Range, whole(&capture), None);
  assert_eq!(analysis.passes, 3);
  assert_eq!(analysis.basis, whole(&capture));
  let roots = row_blocks(&analysis.forest, &analysis.tracks[0].rows[0], analysis.basis, 1000.0);
  assert_eq!(roots.len(), 3);
  for (block, start) in roots.iter().zip([100.0, 400.0, 700.0]) {
    close(block.x, start, "pass start");
    close(block.width, 100.0, "pass width");
    assert_eq!(name_of(&analysis, block), Some("pass"));
  }
  // Zoomed onto the second pass, its phases are drawn as in the frame view.
  let zoomed = row_blocks(
    &analysis.forest,
    &analysis.tracks[0].rows[1],
    TimeRange {
      start_ms: 40.0,
      end_ms: 50.0,
    },
    1000.0,
  );
  let layout = zoomed
    .iter()
    .find(|block| name_of(&analysis, block) == Some("layout_update"))
    .unwrap();
  close(layout.x, 20.0, "layout_update start");
  close(layout.width, 580.0, "layout_update width");
}

#[test]
fn narrow_spans_merge_so_a_row_stays_within_its_width() {
  const WIDTH: f32 = 1200.0;
  let capture = Arc::new(synthetic_capture(2000, 20));
  let analysis = Analysis::new(&capture, AnalysisScope::Range, whole(&capture), None);
  assert_eq!(analysis.passes, 2000);
  assert!(analysis.forest.len() > 60_000, "{}", analysis.forest.len());
  let mut drawn = Vec::new();
  for track in &analysis.tracks {
    let mut track_blocks = 0;
    for row in &track.rows {
      let blocks = row_blocks(&analysis.forest, row, analysis.basis, WIDTH);
      assert!(blocks.len() as f32 <= WIDTH / CELL_PX, "{} blocks", blocks.len());
      for block in &blocks {
        assert!(block.x >= 0.0 && block.x + block.width <= WIDTH + 0.01, "{block:?}");
      }
      // Blocks of a row never overlap.
      for pair in blocks.windows(2) {
        assert!(pair[0].x + pair[0].width <= pair[1].x + 0.01, "{pair:?}");
      }
      track_blocks += blocks.len();
    }
    drawn.push(track_blocks);
  }
  // 60 Hz passes a fraction of a pixel apart merge into a run per row; the
  // scopes, 12 px apart, stay one block each.
  assert_eq!(drawn[0], analysis.tracks[0].rows.len(), "{drawn:?}");
  assert!(drawn.iter().sum::<usize>() < 200, "{drawn:?}");
  // Zoomed to 50 ms, the spans there are their own blocks again.
  let view = TimeRange {
    start_ms: 1000.0,
    end_ms: 1050.0,
  };
  let blocks = row_blocks(&analysis.forest, &analysis.tracks[0].rows[1], view, WIDTH);
  assert!(blocks.iter().filter(|block| block.span.is_some()).count() >= 4);
}

#[test]
fn application_scopes_get_a_track_per_lane_and_are_hit_there() {
  let capture = Arc::new(synthetic_capture(100, 2));
  let analysis = Analysis::new(&capture, AnalysisScope::Range, whole(&capture), None);
  assert_eq!(
    analysis.tracks.iter().map(|track| track.track).collect::<Vec<_>>(),
    SpanTrack::ALL
  );
  let geometry = ChartGeometry::of(&analysis);
  let (worker_top, worker_rows) = geometry.tracks[2];
  assert_eq!(worker_rows, 2);
  // The first worker `load` runs 10..40 ms and holds `decode` 15..35 ms.
  let view = TimeRange {
    start_ms: 0.0,
    end_ms: 100.0,
  };
  let at = |ms: f32| ms * 10.0;
  let name = |found: Option<u32>| found.map(|span| analysis.span(span).name);
  assert_eq!(
    name(hit(&analysis, &geometry, view, 1000.0, at(12.0), worker_top + 2.0)),
    Some("load")
  );
  assert_eq!(
    name(hit(
      &analysis,
      &geometry,
      view,
      1000.0,
      at(20.0),
      worker_top + ROW_HEIGHT + 2.0
    )),
    Some("decode")
  );
  let (ui_top, _) = geometry.tracks[1];
  assert_eq!(
    name(hit(&analysis, &geometry, view, 1000.0, at(11.5), ui_top + 2.0)),
    Some("click")
  );
  // A track header is not a row.
  assert_eq!(
    hit(&analysis, &geometry, view, 1000.0, at(12.0), worker_top - 2.0),
    None
  );
  // The pass at 10 ms on the frames track.
  assert_eq!(
    name(hit(
      &analysis,
      &geometry,
      view,
      1000.0,
      at(11.0),
      geometry.tracks[0].0 + 2.0
    )),
    Some("pass")
  );
  assert_eq!(track_label(&analysis.tracks[2]), "worker lane · application scopes");
}

#[test]
fn zoom_and_pan_stay_inside_the_basis() {
  let capture = three_passes();
  let analysis = Analysis::new(&capture, AnalysisScope::Range, whole(&capture), None);
  let mut state = AnalysisState::default();
  // Halved around the middle.
  state.zoom(&analysis, 0.5, 0.5);
  assert_eq!(
    state.view_of(&analysis),
    TimeRange {
      start_ms: 25.0,
      end_ms: 75.0
    }
  );
  // Around the pointer: the time under it stays put.
  state.zoom(&analysis, 0.5, 0.0);
  assert_eq!(
    state.view_of(&analysis),
    TimeRange {
      start_ms: 25.0,
      end_ms: 50.0
    }
  );
  let view = state.view_of(&analysis);
  state.pan(&analysis, view, 1000.0);
  assert_eq!(
    state.view_of(&analysis),
    TimeRange {
      start_ms: 75.0,
      end_ms: 100.0
    }
  );
  state.pan(&analysis, view, -1000.0);
  assert_eq!(state.view_of(&analysis).start_ms, 0.0);
  for _ in 0..100 {
    state.zoom(&analysis, 0.1, 0.5);
  }
  assert!((state.view_of(&analysis).duration_ms() - MIN_VIEW_MS).abs() < 1e-9);
  state.zoom(&analysis, 1e12, 0.5);
  assert_eq!(state.view_of(&analysis), analysis.basis);
  // Another analysis starts unzoomed.
  let other = Analysis::new(&capture, AnalysisScope::Frame, whole(&capture), Some(1));
  state.zoom(&analysis, 0.5, 0.5);
  assert_eq!(state.view_of(&other), other.basis);
}

#[test]
fn tables_total_the_pass_time_of_the_range() {
  let capture = Arc::new(synthetic_capture(50, 5));
  let range = TimeRange {
    start_ms: 100.0,
    end_ms: 500.0,
  };
  let analysis = Analysis::new(&capture, AnalysisScope::Range, range, None);
  let pass_time_ns: u128 = capture
    .passes_in(range)
    .iter()
    .map(|sample| pass_of(sample).unwrap().total.as_nanos())
    .sum();
  assert_eq!(analysis.passes, capture.passes_in(range).len());
  assert!((analysis.pass_time_ms - pass_time_ns as f64 / 1e6).abs() < 1e-6);
  for bottom_up in [false, true] {
    let tree = analysis.table(bottom_up, SpanTrack::Frames, CallSort::Total, true);
    assert_eq!(u128::from(tree.total_ns), pass_time_ns, "bottom_up {bottom_up}");
    let self_sum: u64 = if bottom_up {
      tree.roots.iter().map(|root| tree.nodes[*root as usize].self_ns).sum()
    } else {
      tree.nodes.iter().map(|node| node.self_ns).sum()
    };
    assert_eq!(u128::from(self_sum), pass_time_ns, "bottom_up {bottom_up}");
    // Made once per sort.
    assert!(Arc::ptr_eq(
      &tree,
      &analysis.table(bottom_up, SpanTrack::Frames, CallSort::Total, true)
    ));
  }
  let tree = analysis.table(false, SpanTrack::Frames, CallSort::Total, true);
  let pass = &tree.nodes[tree.roots[0] as usize];
  assert_eq!(pass.name, "pass");
  assert_eq!(pass.count as usize, analysis.passes);
  assert!((tree.percent(pass.total_ns) - 100.0).abs() < 1e-9);
}

/// What a mounted analysis shows, and the name of what is selected in it.
struct Shown {
  analysis: Signal<AnalysisState>,
  picked: Signal<Option<&'static str>>,
}

/// `ProfilerAnalysis` mounted alone on `capture`'s whole range, with what is shown of it and the passes its
/// clicks select.
fn mounted(capture: Arc<ProfilerCapture>, selected: Option<u64>) -> (App, Tree, Shown, Arc<Mutex<Vec<u64>>>) {
  mounted_at(capture, selected, 600)
}

/// As `mounted`, in a panel `height` high.
fn mounted_at(
  capture: Arc<ProfilerCapture>,
  selected: Option<u64>,
  height: u32,
) -> (App, Tree, Shown, Arc<Mutex<Vec<u64>>>) {
  let picked = Arc::new(Mutex::new(Vec::new()));
  let state = Shown {
    analysis: Signal::new(AnalysisState::default()),
    picked: Signal::new(None),
  };
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(1000, height);
  tree.mount_root::<ProfilerAnalysis>(
    &mut app,
    AnalysisProps {
      range: capture.whole(),
      capture,
      selected,
      state: state.analysis.clone(),
      picked: state.picked.clone(),
      on_select_pass: Some(Arc::new({
        let picked = picked.clone();
        move |sequence| picked.lock().unwrap().push(sequence)
      })),
    },
  );
  tree.pass_headless(&mut app);
  tree.pass_headless(&mut app);
  (app, tree, state, picked)
}

fn bounds_of(tree: &mut Tree, id: &str) -> Option<crate::core::ElementRect> {
  tree
    .find_element(|element: crate::node::ElementRef<'_>| element.id() == Some(id))
    .map(|element| element.bounds())
}

fn texts_of(tree: &mut Tree, id: &str) -> Vec<String> {
  fn collect(element: crate::node::ElementRef<'_>, texts: &mut Vec<String>) {
    if let Some(text) = element.text_content() {
      texts.push(text.to_owned());
    }
    for child in element.children() {
      collect(child, texts);
    }
  }
  let mut texts = Vec::new();
  if let Some(element) = tree.get_element_by_id(id) {
    collect(element, &mut texts);
  }
  texts
}

fn click(tree: &mut Tree, x: f32, y: f32) {
  tree.mouse_down(x, y, MouseButton::Left);
  tree.mouse_up(x, y, MouseButton::Left);
}

#[test]
fn the_chart_hovers_selects_zooms_and_pans() {
  let (mut app, mut tree, state, picked) = mounted(three_passes(), None);
  let chart = bounds_of(&mut tree, CHART_ID).expect("the chart");
  // The board's room each side of the 1000 px panel.
  close(chart.width, 1000.0 - PADDING * 2.0, "chart width");
  let view_text = |tree: &mut Tree| texts_of(tree, VIEW_ID).concat();
  assert_eq!(view_text(&mut tree), "+0ms – +100ms (100.00ms)");

  // layout_update of pass 2 runs 40.2..46 ms, on the frames track's second row.
  let x = chart.x + chart.width * 0.43;
  let y = chart.y + TRACK_HEADER_HEIGHT + ROW_HEIGHT * 1.5;
  tree.mouse_move(x, y);
  tree.pass_headless(&mut app);
  // What the pointer says is one text of three lines.
  let said = texts_of(&mut tree, TOOLTIP_ID).concat();
  let tooltip: Vec<&str> = said.lines().collect();
  assert_eq!(tooltip[0], "layout_update");
  // 5.8 ms less ui_rebuild 2.6, layout_compute 1.8 and component_after_layout 0.6.
  assert_eq!(tooltip[1], "total 5.80ms · self 0.80ms");
  assert_eq!(tooltip[2], "pass #2 · +40ms");

  click(&mut tree, x, y);
  tree.pass_headless(&mut app);
  assert_eq!(*picked.lock().unwrap(), [2]);
  // Its name is what the tables and the sidebar pick out.
  assert_eq!(state.picked.get_untracked(), Some("layout_update"));
  assert_eq!(
    texts_of(&mut tree, SELECTION_ID),
    [
      "Selected span",
      "layout_update",
      "Total",
      "5.80ms",
      "Self",
      "0.80ms",
      "Parent",
      "pass",
      "Origin",
      "phase · pass #2 · +40ms"
    ]
  );

  // The wheel zooms in around the pointer.
  tree.scroll(x, y, 0.0, 100.0, ScrollPhase::Scroll);
  tree.pass_headless(&mut app);
  let zoomed = view_text(&mut tree);
  assert!(zoomed.ends_with("(67.03ms)"), "{zoomed}");

  // A drag to the left pans later in time, and selects nothing.
  tree.mouse_down(x, y, MouseButton::Left);
  tree.mouse_move(x - 100.0, y);
  tree.mouse_move(x - 200.0, y);
  tree.mouse_up(x - 200.0, y, MouseButton::Left);
  tree.pass_headless(&mut app);
  let panned = view_text(&mut tree);
  assert_ne!(panned, zoomed);
  assert!(panned.ends_with("(67.03ms)"), "{panned}");
  assert_eq!(*picked.lock().unwrap(), [2]);

  // Leaving the chart hides the tooltip.
  tree.mouse_move(x, chart.y + chart.height + 40.0);
  tree.pass_headless(&mut app);
  assert!(bounds_of(&mut tree, TOOLTIP_ID).is_none());
}

#[test]
fn a_chart_higher_than_its_room_is_dragged_down_its_rows() {
  // Three tracks, the frames' four rows, the UI lane's one and the worker lane's two, in a panel too low for
  // them.
  let capture = Arc::new(synthetic_capture(100, 2));
  let analysis = Analysis::new(&capture, AnalysisScope::Range, capture.whole(), None);
  let height = ChartGeometry::of(&analysis).height;
  let (mut app, mut tree, state, picked) = mounted_at(capture, None, 300);
  let chart = bounds_of(&mut tree, CHART_ID).expect("the chart");
  let end = bounds_of(&mut tree, SELECTION_ID).expect("the line under the chart").y;
  assert!(chart.y + height > end + 10.0, "the chart is higher than its room");
  let drag = |tree: &mut Tree, app: &mut App, from: f32, to: f32| {
    let x = chart.x + 500.0;
    tree.mouse_down(x, from, MouseButton::Left);
    tree.mouse_move(x, (from + to) / 2.0);
    tree.mouse_move(x, to);
    tree.mouse_up(x, to, MouseButton::Left);
    tree.pass_headless(app);
    tree.pass_headless(app);
  };

  // Dragged up by 20, it is 20 higher, and shows all of its time as before.
  let y = chart.y + 40.0;
  drag(&mut tree, &mut app, y, y - 20.0);
  let moved = bounds_of(&mut tree, CHART_ID).unwrap();
  close(chart.y - moved.y, 20.0, "the chart goes up with the pointer");
  assert_eq!(state.analysis.get_untracked().scroll_y, 20.0);
  assert!(
    state.analysis.get_untracked().view.is_none(),
    "a drag straight up zooms nothing"
  );
  assert!(picked.lock().unwrap().is_empty(), "a drag selects nothing");

  // Dragged further than it is high, it stops with its last row at the room's end.
  drag(&mut tree, &mut app, y, y - 30.0);
  let moved = bounds_of(&mut tree, CHART_ID).unwrap();
  close(moved.y + height, end, "the last row at the room's end");
  // What was under the room's end is hit now: the worker lane's `decode`, on its second row.
  let geometry = ChartGeometry::of(&analysis);
  let (worker_top, _) = geometry.tracks[2];
  let x = chart.x + chart.width * 0.015;
  tree.mouse_move(x, moved.y + worker_top + ROW_HEIGHT * 1.5);
  tree.pass_headless(&mut app);
  assert!(
    texts_of(&mut tree, TOOLTIP_ID).concat().starts_with("decode"),
    "{:?}",
    texts_of(&mut tree, TOOLTIP_ID)
  );

  // And back down to its first row.
  drag(&mut tree, &mut app, y, y + 60.0);
  close(
    bounds_of(&mut tree, CHART_ID).unwrap().y,
    chart.y,
    "back at its first row",
  );
  assert_eq!(state.analysis.get_untracked().scroll_y, 0.0);
}

fn shows(tree: &mut Tree, text: &str) -> bool {
  tree
    .find_element(|element: crate::node::ElementRef<'_>| element.text_content() == Some(text))
    .is_some()
}

#[test]
fn the_tables_show_total_self_count_and_percent() {
  let capture = Arc::new(synthetic_capture(30, 4));
  let analysis = Analysis::new(&capture, AnalysisScope::Range, capture.whole(), None);
  let (mut app, mut tree, state, _) = mounted(capture.clone(), None);
  state.analysis.update(|state| state.tab = AnalysisTab::CallTree);
  tree.pass_headless(&mut app);
  tree.pass_headless(&mut app);
  assert!(bounds_of(&mut tree, CHART_ID).is_none());

  let tree_table = analysis.table(false, SpanTrack::Frames, CallSort::Total, true);
  let pass = &tree_table.nodes[tree_table.roots[0] as usize];
  // The row of the passes: total, total %, self, self %, count.
  for text in [
    "pass".to_owned(),
    format_ms(pass.total_ns as f64 / 1e6),
    "100.0%".to_owned(),
    format_ms(pass.self_ns as f64 / 1e6),
    format!("{:.1}%", tree_table.percent(pass.self_ns)),
    "30".to_owned(),
  ] {
    assert!(shows(&mut tree, &text), "the table shows {text}");
  }
  // Open by default to the phases of the passes; deeper rows are closed.
  assert!(shows(&mut tree, "layout_update"));
  assert!(!shows(&mut tree, "ui_commit"));
  // The table's total is the analysis's pass time.
  assert!(shows(
    &mut tree,
    &format!("total {} = pass time of 30 passes", format_ms(analysis.pass_time_ms))
  ));

  state.analysis.update(|state| state.tab = AnalysisTab::BottomUp);
  tree.pass_headless(&mut app);
  tree.pass_headless(&mut app);
  // Every name is a top row; canvas_commands' total is its self time.
  let table = analysis.table(true, SpanTrack::Frames, CallSort::Total, true);
  let commands = table
    .roots
    .iter()
    .map(|root| &table.nodes[*root as usize])
    .find(|node| node.name == "canvas_commands")
    .unwrap();
  assert_eq!(commands.count, 30 * 4);
  assert_eq!(commands.total_ns, commands.self_ns);
  assert!(shows(&mut tree, "canvas_commands"));
  assert!(shows(&mut tree, "120"));
  assert!(shows(&mut tree, "ui_commit"));
}

#[test]
fn a_table_lists_each_open_row_over_its_children_in_the_color_of_its_phase() {
  use crate::app::theme::PaletteColor;
  let capture = three_passes();
  let analysis = Analysis::new(&capture, AnalysisScope::Range, whole(&capture), None);
  let mut state = AnalysisState::default();
  let tree = analysis.table(false, SpanTrack::Frames, CallSort::Total, true);
  let listed = |state: &AnalysisState| {
    calls::lines(&tree, SpanTrack::Frames, false, |index, node| {
      state.expanded(false, SpanTrack::Frames, index, node)
    })
  };
  let lines = listed(&state);
  // The passes and their phases are open; what a phase holds is closed.
  assert_eq!(
    lines
      .iter()
      .map(|line| (line.name, line.depth, line.open))
      .collect::<Vec<_>>(),
    [
      ("pass", 0, true),
      ("layout_update", 1, true),
      ("ui_rebuild", 2, false),
      ("layout_compute", 2, false),
      ("component_after_layout", 2, false),
      ("render", 1, true),
      ("render_encode", 2, false),
      ("render_submit", 2, false),
      ("render_acquire", 2, false),
      ("render_present", 2, false)
    ]
  );
  assert_eq!(lines[0].count, 3);
  assert!((lines[0].total_share - 100.0).abs() < 1e-9);
  // ui_rebuild holds ui_commit: opened, the row is under it.
  let rebuild = lines.iter().find(|line| line.name == "ui_rebuild").unwrap();
  assert_eq!(rebuild.children, 1);
  state.toggle(false, SpanTrack::Frames, rebuild.index);
  let opened = listed(&state);
  let at = opened.iter().position(|line| line.name == "ui_rebuild").unwrap();
  assert!(opened[at].open);
  assert_eq!((opened[at + 1].name, opened[at + 1].depth), ("ui_commit", 3));
  // Closed again, and the passes closed: one row.
  state.toggle(false, SpanTrack::Frames, rebuild.index);
  state.toggle(false, SpanTrack::Frames, lines[0].index);
  assert_eq!(listed(&state).len(), 1);

  // A span of the chart is in the color of its phase, and a scope of the app's in the color of its own.
  let color = |name: &str| {
    analysis
      .forest
      .spans()
      .iter()
      .find(|span| span.name == name)
      .map(|span| color_of(&analysis, span))
  };
  assert_eq!(color("pass"), Some(PaletteColor::MutedForeground));
  assert_eq!(color("ui_commit"), Some(PaletteColor::Chart1));
  assert_eq!(color("layout_compute"), Some(PaletteColor::Chart2));
  assert_eq!(color("render_submit"), Some(PaletteColor::Chart4));
  assert_eq!(color("load"), Some(PaletteColor::Chart5));
}

#[test]
fn a_span_is_shown_with_a_tenth_of_its_length_at_each_side() {
  let capture = three_passes();
  let analysis = Analysis::new(&capture, AnalysisScope::Range, whole(&capture), None);
  let pass = analysis
    .forest
    .spans()
    .iter()
    .position(|span| span.name == "pass" && span.sample == Some(2))
    .unwrap();
  let mut state = AnalysisState::default();
  state.zoom_to(&analysis, pass as u32);
  // The second pass runs 40..50 ms.
  let view = state.view_of(&analysis);
  assert!((view.start_ms - 39.0).abs() < 1e-9, "{view:?}");
  assert!((view.end_ms - 51.0).abs() < 1e-9, "{view:?}");
}
