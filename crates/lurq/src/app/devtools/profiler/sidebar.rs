//! What is known of the selected pass, beside the timeline: what it took and why it ran, and under that its
//! spans, what made its components render, the inputs its frame presented, and everything it counted.

use super::{
  Budget, ProfilerState, ProfilerTab,
  capture::{
    self, CaptureScreenshot, DetailSection, FRAME_BUDGET_MS, PhaseRow, ProfilerCapture, format_duration, format_ms, ms,
    pass_of,
  },
  format_offset, phase_color,
  timeline::screenshot_image,
};
use crate::{
  app::{
    devtools::{
      changes,
      kit::{FILL, line, mono, sans},
    },
    profiler::{InputDispatchSample, PassSample, ProfileSample, RenderCause, RenderSource},
    theme::{PaletteColor, RadiusSize, TypographyStyle},
  },
  components::{Column, Row, Spacer, Text},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::Element,
  ui::{
    Badge, BadgePart, BadgeSize, BadgeVariant, EmptyState, EmptyStateSize, Length, ScrollArea, Tab, Tabs, TabsPart,
    TabsVariant, icons,
  },
};

/// The sizes the board gives the sidebar (`pencil/lurq-devtools.pen`, Profiler · dark).
pub(super) const WIDTH: f32 = 340.0;
const PADDING: f32 = 16.0;
const HEADER_HEIGHT: f32 = 48.0;
const TABS_HEIGHT: f32 = 40.0;
const SUMMARY_LABEL: f32 = 104.0;
const LABELS_HEIGHT: f32 = 28.0;
/// The most the screenshot of the selected pass is high.
const PREVIEW_MAX_HEIGHT: f32 = 240.0;
/// Spans the sidebar lists for one pass; the rest are counted.
const MAX_PHASE_ROWS: usize = 256;

/// The element id of the sidebar's screenshot preview.
pub(crate) const PREVIEW_ID: &str = "devtools-profiler-screenshot-preview";

/// The lists of the selected pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, crate::DevtoolsInspectable)]
pub(crate) enum DetailTab {
  #[default]
  Phases,
  Causes,
  Input,
  Counters,
}

pub(super) fn sidebar(capture: &ProfilerCapture, state: &ProfilerState, tab: &ProfilerTab) -> Element {
  let selected = state
    .selected
    .and_then(|sequence| capture.pass(sequence))
    .and_then(|sample| pass_of(sample).map(|pass| (sample, pass)));
  // The screenshot under the pointer, else what the selected pass showed.
  let preview = state
    .hovered_screenshot
    .and_then(|index| capture.screenshots.get(index))
    .or_else(|| state.selected.and_then(|sequence| capture.screenshot_for(sequence)));
  let mut sidebar = Column::new()
    .width(WIDTH)
    .height(FILL)
    .background(PaletteColor::Card)
    .foreground(PaletteColor::CardForeground)
    .id("devtools-profiler-sidebar");
  let Some((sample, pass)) = selected else {
    if let Some(shot) = preview {
      sidebar = sidebar.child(screenshot(capture, shot));
    }
    return sidebar
      .child(changes::centred(
        EmptyState::new("No pass selected")
          .description("Press a bar of the frames to see what its pass took.")
          .icon(icons::PULSE)
          .size(EmptyStateSize::Sm)
          .into(),
      ))
      .into();
  };

  sidebar = sidebar.child(header(capture, sample, pass));
  if let Some(shot) = preview {
    sidebar = sidebar.child(screenshot(capture, shot));
  }
  let phases = capture::phase_rows(pass);
  let causes = capture::causes(&capture.updates_of(sample));
  let inputs = pass
    .frame_id
    .map(|frame_id| capture.inputs_presented_by(&sample.window, frame_id))
    .unwrap_or_default();
  let tabs = Tabs::new(
    tab.detail.clone(),
    [
      Tab::new(DetailTab::Phases, "Phases").count(phases.len()),
      Tab::new(DetailTab::Causes, "Causes").count(causes.len()),
      Tab::new(DetailTab::Input, "Input").count(inputs.len()),
      Tab::new(DetailTab::Counters, "Counters"),
    ],
  )
  .variant(TabsVariant::Line)
  .label("Lists of the selected pass")
  .id("devtools-profiler-detail")
  .style(TabsPart::List, |s| s.height(TABS_HEIGHT).padding_x(PADDING).gap(20.0))
  .style(TabsPart::Tab, |s| s.height(TABS_HEIGHT).gap(6.0))
  .style(TabsPart::Label, |s| s.font_size(13.0))
  .style(TabsPart::Count, |s| s.font_size(12.0));
  // The span selected in the chart or in a table is picked out among the pass's own.
  let picked = tab.picked.get();
  let list = match tab.detail.get() {
    DetailTab::Phases => self::phases(&phases, pass.dropped_spans, picked),
    DetailTab::Causes => self::causes(&causes),
    DetailTab::Input => self::inputs(&inputs),
    DetailTab::Counters => counters(&capture::pass_detail(sample, pass)),
  };
  sidebar
    .child(summary(capture, pass, sample))
    .child(tabs)
    .child(Column::new().width(FILL).flex(1.0).border_top(line()).child(list))
    .into()
}

fn quiet(text: Text) -> Text {
  text.color(PaletteColor::MutedForeground)
}

fn code(text: &str, weight: FontWeight) -> Text {
  mono(text, 12.0, weight)
}

/// Which pass it is, whether it was over the budget, and when it began.
fn header(capture: &ProfilerCapture, sample: &ProfileSample, pass: &PassSample) -> Row {
  let mut header = Row::new()
    .width(FILL)
    .height(HEADER_HEIGHT)
    .padding_horizontal(PADDING)
    .spacing(10.0)
    .align_items(Alignment::Center)
    .border_bottom(line())
    .child(mono(&format!("Pass #{}", sample.sequence), 15.0, FontWeight::SemiBold).id("devtools-profiler-pass"));
  header = match Budget::of(ms(pass.total)) {
    Budget::Within => header,
    Budget::Over => header.child(
      Badge::new("over budget")
        .variant(BadgeVariant::Warning)
        .size(BadgeSize::Sm),
    ),
    Budget::OverTwice => header.child(
      Badge::new("over 2× budget")
        .variant(BadgeVariant::Destructive)
        .size(BadgeSize::Sm),
    ),
  };
  header.child(Spacer::new().flex(1.0)).child(quiet(code(
    &format_offset(capture, sample.started_ms),
    FontWeight::Normal,
  )))
}

/// `shot` at the sidebar's width, aspect kept, with its window, its frame and its time.
fn screenshot(capture: &ProfilerCapture, shot: &CaptureScreenshot) -> Column {
  let taken = &shot.screenshot;
  let width = WIDTH - PADDING * 2.0;
  let height = (width * taken.height as f32 / taken.width.max(1) as f32).min(PREVIEW_MAX_HEIGHT);
  Column::new()
    .id(PREVIEW_ID)
    .width(FILL)
    .padding_horizontal(PADDING)
    .padding_vertical(12.0)
    .spacing(8.0)
    .border_bottom(line())
    .child(
      Row::new()
        .width(width)
        .height(height)
        .rounded(RadiusSize::Lg)
        .clip()
        .background(PaletteColor::Muted)
        .child(screenshot_image(shot)),
    )
    .child(quiet(mono(
      &format!(
        "{} · frame {} · {}",
        taken.window,
        taken.frame_id,
        format_offset(capture, shot.at_ms)
      ),
      11.0,
      FontWeight::Normal,
    )))
}

/// What the pass took, whether its frame was presented and on time, and why it ran.
fn summary(capture: &ProfilerCapture, pass: &PassSample, sample: &ProfileSample) -> Column {
  let row = |label: &str, value: Element| {
    Row::new()
      .width(FILL)
      .min_height(28.0)
      .align_items(Alignment::Center)
      .child(quiet(sans(label, 12.0, FontWeight::Normal)).width(SUMMARY_LABEL))
      .child(value)
  };
  let said = |text: &str| -> Element { code(text, FontWeight::Medium).into() };
  let total = ms(pass.total);
  let duration = Row::new()
    .spacing(8.0)
    .align_items(Alignment::Center)
    .child(code(&format_ms(total), FontWeight::Medium).color(Budget::of(total).color(PaletteColor::Foreground)))
    .child(quiet(code(
      &format!("of {} budget", format_ms(FRAME_BUDGET_MS)),
      FontWeight::Normal,
    )));
  let yes_no = |yes: bool| said(if yes { "yes" } else { "no" });
  let missed = pass.present.map_or(0, |present| {
    if present.missed_vsync() {
      present.missed_vsyncs
    } else {
      0
    }
  });
  let mut reasons = Column::new().spacing(4.0).align_items(Alignment::Start);
  let mut any = false;
  for reason in pass.reasons.names() {
    any = true;
    reasons = reasons.child(
      Badge::new(reason)
        .variant(BadgeVariant::Outline)
        .size(BadgeSize::Sm)
        .style(BadgePart::Label, |s| {
          s.typography(TypographyStyle::Mono).font_size(11.0)
        }),
    );
  }
  let mut summary = Column::new()
    .width(FILL)
    .padding_top(8.0)
    .padding_horizontal(PADDING)
    .padding_bottom(12.0)
    .id("devtools-profiler-summary")
    .child(row("Duration", duration.into()))
    .child(row(
      "GPU",
      match pass.gpu {
        Some(gpu) => said(&format_duration(gpu.total())),
        None => quiet(code("–", FontWeight::Normal)).into(),
      },
    ))
    .child(row("Presented", yes_no(pass.rendered)))
    .child(row(
      "Missed vsync",
      match missed {
        0 => yes_no(false),
        1 => yes_no(true),
        missed => said(&format!("yes · {missed}")),
      },
    ))
    .child(row("Window", said(&sample.window)));
  if pass.screenshot {
    summary = summary.child(row(
      "Screenshot",
      quiet(code("copied: render includes it", FontWeight::Normal)).into(),
    ));
  }
  // The reasons are one under the other: their label is beside the first.
  summary = summary.child(
    Row::new()
      .width(FILL)
      .padding_vertical(5.0)
      .child(quiet(sans("Reasons", 12.0, FontWeight::Normal)).width(SUMMARY_LABEL))
      .child(if any {
        Element::from(reasons)
      } else {
        quiet(code("–", FontWeight::Normal)).into()
      }),
  );
  if pass.rendered && pass.gpu.is_none() {
    summary = summary.child(quiet(sans(
      &format!("No GPU time: {}", capture.gpu_missing_reason()),
      11.0,
      FontWeight::Normal,
    )));
  }
  summary
}

fn scrolled(label: &str, rows: Column) -> Element {
  ScrollArea::new().label(label).height(Length::Fill).child(rows).into()
}

/// A line that says a list has nothing, or what it leaves out.
fn note(text: &str) -> Row {
  Row::new()
    .width(FILL)
    .min_height(28.0)
    .padding_horizontal(PADDING)
    .align_items(Alignment::Center)
    .child(quiet(sans(text, 12.0, FontWeight::Normal)))
}

/// The pass's spans, nested by depth, each with the share of the pass it took.
fn phases(rows: &[PhaseRow], dropped_spans: u32, picked: Option<&'static str>) -> Element {
  let pass_ms = rows.first().map_or(0.0, |row| row.duration_ms).max(f64::EPSILON);
  let mut list = Column::new()
    .width(FILL)
    .padding_vertical(6.0)
    .id("devtools-profiler-phases");
  if rows.is_empty() {
    list = list.child(note("No spans recorded"));
  }
  // A component is in the color of the phase it is in: the colors of the rows a row is under, the pass's first.
  let mut over: Vec<PaletteColor> = Vec::new();
  for row in rows.iter().take(MAX_PHASE_ROWS) {
    let depth = usize::from(row.depth);
    over.truncate(depth);
    let color = phase_color(row.name).unwrap_or_else(|| match over.last() {
      // The pass's own color is not its phases'.
      Some(color) if depth > 1 => color.clone(),
      _ => PaletteColor::Chart1,
    });
    over.resize(depth, color.clone());
    over.push(color.clone());
    let top = row.depth == 0;
    let share = (row.duration_ms / pass_ms).clamp(0.0, 1.0) as f32;
    let name = code(row.name, if top { FontWeight::Medium } else { FontWeight::Normal });
    let time = code(&format_ms(row.duration_ms), FontWeight::Normal);
    let mut line = Row::new()
      .width(FILL)
      .height(24.0)
      .padding_horizontal(PADDING)
      .spacing(10.0)
      .align_items(Alignment::Center)
      // The name gives way to the bar and the time: a long one deep in the pass is cut.
      .child(
        changes::cut(name)
          .flex(1.0)
          .padding_left(f32::from(row.depth) * 12.0),
      )
      .child(
        Row::new()
          .size(56.0, 6.0)
          .rounded(3.0)
          .background(PaletteColor::Muted)
          .child(
            Row::new()
              .width((56.0 * share).max(2.0))
              .height(FILL)
              .rounded(3.0)
              .background(color),
          ),
      )
      .child(
        Row::new()
          .width(60.0)
          .justify(Justify::End)
          .child(if top { time } else { quiet(time) }),
      );
    if picked == Some(row.name) && !top {
      line = line
        .background(PaletteColor::Highlight)
        .foreground(PaletteColor::HighlightForeground);
    }
    list = list.child(line);
  }
  let hidden = rows.len().saturating_sub(MAX_PHASE_ROWS) as u64 + u64::from(dropped_spans);
  if hidden > 0 {
    list = list.child(note(&format!("{hidden} more spans not shown")));
  }
  scrolled("Phases of the selected pass", list)
}

/// The labels over a list of the sidebar: each as wide as its column, the last taking what is left.
fn labels(columns: &[(&str, Option<f32>, bool)]) -> Row {
  let mut labels = Row::new()
    .width(FILL)
    .height(LABELS_HEIGHT)
    .padding_horizontal(PADDING)
    .spacing(8.0)
    .align_items(Alignment::Center)
    .border_bottom(line());
  for (label, width, end) in columns {
    let text = quiet(sans(label, 12.0, FontWeight::Medium));
    let mut cell = Row::new().child(text);
    if *end {
      cell = cell.justify(Justify::End);
    }
    labels = labels.child(match width {
      Some(width) => cell.width(*width),
      None => cell.flex(1.0),
    });
  }
  labels
}

const CAUSE_COMPONENT: f32 = 124.0;
const CAUSE_TRIGGER: f32 = 64.0;

/// What made the pass's components render, one row per distinct cause: the component, then what changed and
/// the type of its value, and for a memo what made it recompute, under it.
fn causes(causes: &[&RenderCause]) -> Element {
  let source = |source: &RenderSource, via: bool| {
    let trigger = code(source.trigger.name(), FontWeight::Normal);
    Row::new()
      .spacing(8.0)
      .align_items(Alignment::Center)
      .child(
        Row::new()
          .width(CAUSE_TRIGGER)
          .child(if via { quiet(trigger) } else { trigger }),
      )
      .child(changes::cut(quiet(code(source.value_type.unwrap_or("–"), FontWeight::Normal))).flex(1.0))
      .flex(1.0)
  };
  let mut list = Column::new()
    .width(FILL)
    .padding_vertical(4.0)
    .id("devtools-profiler-causes");
  if causes.is_empty() {
    list = list.child(note(
      "No render causes: record with Components on to see what made each component render.",
    ));
  }
  for cause in causes {
    list = list.child(
      Row::new()
        .width(FILL)
        .height(28.0)
        .padding_horizontal(PADDING)
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(changes::cut(code(cause.component, FontWeight::Medium)).width(CAUSE_COMPONENT))
        .child(source(&cause.source, false)),
    );
    if let Some(via) = &cause.via {
      list = list.child(
        Row::new()
          .width(FILL)
          .height(22.0)
          .padding_horizontal(PADDING)
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(
            Row::new()
              .width(CAUSE_COMPONENT)
              .padding_left(12.0)
              .child(quiet(code("via", FontWeight::Normal))),
          )
          .child(source(via, true)),
      );
    }
  }
  Column::new()
    .size(FILL, FILL)
    .child(labels(&[
      ("Component", Some(CAUSE_COMPONENT), false),
      ("Trigger", Some(CAUSE_TRIGGER), false),
      ("Type", None, false),
    ]))
    .child(
      Column::new()
        .width(FILL)
        .flex(1.0)
        .child(scrolled("Render causes of the selected pass", list)),
    )
    .into()
}

const INPUT_TIME: f32 = 68.0;

/// One row per input the pass's frame presented: what it was, how long it waited, how long its dispatch took,
/// and how long after it the frame was presented.
fn inputs(inputs: &[&InputDispatchSample]) -> Element {
  let time = |text: String, weight: FontWeight| {
    Row::new()
      .width(INPUT_TIME)
      .justify(Justify::End)
      .child(code(&text, weight))
  };
  let mut list = Column::new()
    .width(FILL)
    .padding_vertical(4.0)
    .id("devtools-profiler-inputs");
  if inputs.is_empty() {
    list = list.child(note("The frame of this pass presented no input."));
  }
  for input in inputs {
    list = list.child(
      Row::new()
        .width(FILL)
        .height(28.0)
        .padding_horizontal(PADDING)
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(
          Row::new()
            .flex(1.0)
            .spacing(8.0)
            .align_items(Alignment::Center)
            .child(code(input.kind.name(), FontWeight::Medium))
            .child(quiet(code(&format!("#{}", input.interaction_id), FontWeight::Normal))),
        )
        .child(time(format_duration(input.queue_delay), FontWeight::Normal))
        .child(time(format_duration(input.total), FontWeight::Normal))
        .child(time(
          input.input_to_present.map_or_else(|| "–".to_owned(), format_duration),
          FontWeight::Medium,
        )),
    );
  }
  Column::new()
    .size(FILL, FILL)
    .child(labels(&[
      ("Input", None, false),
      ("Queue", Some(INPUT_TIME), true),
      ("Dispatch", Some(INPUT_TIME), true),
      ("Present", Some(INPUT_TIME), true),
    ]))
    .child(
      Column::new()
        .width(FILL)
        .flex(1.0)
        .child(scrolled("Inputs the selected pass presented", list)),
    )
    .into()
}

/// Everything the sample holds about the pass, its frame profile included, group by group. A value that says
/// nothing happened steps back.
fn counters(sections: &[DetailSection]) -> Element {
  let mut list = Column::new()
    .width(FILL)
    .padding_bottom(8.0)
    .id("devtools-profiler-counters");
  for section in sections {
    list = list.child(
      Row::new()
        .width(FILL)
        .height(30.0)
        .padding_top(10.0)
        .padding_horizontal(PADDING)
        .align_items(Alignment::Center)
        .child(quiet(sans(section.title, 11.0, FontWeight::SemiBold))),
    );
    for (label, value) in &section.rows {
      let nothing = matches!(value.as_str(), "false" | "0" | "-" | "unavailable" | "0.00ms");
      let said = code(value, FontWeight::Normal);
      list = list.child(
        Row::new()
          .width(FILL)
          .height(24.0)
          .padding_horizontal(PADDING)
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(changes::cut(quiet(code(label, FontWeight::Normal))).flex(1.0))
          .child(if nothing { quiet(said) } else { said }),
      );
    }
  }
  scrolled("Counters of the selected pass", list)
}
