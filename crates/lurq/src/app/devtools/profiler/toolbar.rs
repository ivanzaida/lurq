//! The toolbar of the Profiler tab: what starts and stops a recording, what a recording takes, the status of a
//! running one or the last error, and the files.

use super::{
  ProfilerState, ProfilerTab, Recording,
  capture::{format_bytes, format_span},
};
use crate::{
  app::{
    devtools::{
      changes,
      kit::{self, mono},
    },
    profiler::ProfilingHandle,
    theme::PaletteColor,
  },
  components::{Row, Spacer},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  ui::{Button, ButtonSize, ButtonVariant, Icon, Recipe, Separator, Toggle, TogglePart, ToggleSize, icons},
};

pub(super) fn toolbar(state: &ProfilerState, tab: &ProfilerTab, handle: Option<ProfilingHandle>) -> Row {
  let recording = state.recording();
  let mut row = kit::toolbar().child(record(recording, tab, handle));
  row = match &state.recording {
    Some(running) => row.child(
      mono(&status_line(running), 12.0, FontWeight::Normal)
        .color(PaletteColor::Destructive)
        .id("devtools-profiler-status"),
    ),
    // What a recording takes is read when it starts: there is nothing to set while one runs.
    None => row
      .child(Separator::vertical().length(16.0))
      .child(option(
        "Include DevTools",
        icons::LAYERS,
        state.include_devtools,
        |state, on| state.include_devtools = on,
        &tab.state,
      ))
      .child(option(
        "Components",
        icons::COMPONENT,
        state.detailed,
        |state, on| state.detailed = on,
        &tab.state,
      ))
      .child(option(
        "Screenshots",
        icons::IMAGE,
        state.screenshots,
        |state, on| state.screenshots = on,
        &tab.state,
      )),
  };
  row = match &state.error {
    // It takes the room there is and is cut at its end: a long message moves no button.
    Some(error) => row.child(
      Row::new()
        .flex(1.0)
        .padding_left(8.0)
        .spacing(6.0)
        .align_items(Alignment::Center)
        .child(
          Icon::new(icons::ALERT_CIRCLE)
            .size(14.0)
            .color(PaletteColor::Destructive),
        )
        .child(
          changes::cut(
            mono(error, 12.0, FontWeight::Normal)
              .color(PaletteColor::Destructive)
              .id("devtools-profiler-error"),
          )
          .flex(1.0),
        ),
    ),
    None => row.child(Spacer::new().flex(1.0)),
  };
  let (cleared, ticker) = (tab.state.clone(), tab.ticker.clone());
  files(row, state, &tab.state).child(
    button("Clear", icons::TRASH)
      .on_click(move |_| {
        ticker.stop();
        cleared.update(ProfilerState::clear);
      })
      .id("devtools-profiler-clear"),
  )
}

/// Start recording, and Stop recording while one runs.
fn record(recording: bool, tab: &ProfilerTab, handle: Option<ProfilingHandle>) -> Toggle {
  let (state, ticker) = (tab.state.clone(), tab.ticker.clone());
  Toggle::new(if recording { "Stop recording" } else { "Start recording" })
    .icon(if recording { icons::SQUARE } else { icons::DOT_CIRCLE })
    .size(ToggleSize::Xs)
    .rules(&record_toggle())
    .on(recording)
    .on_change(move |_| {
      if recording {
        ticker.stop();
        state.update(ProfilerState::stop);
        return;
      }
      let Some(handle) = &handle else {
        state.update(|state| state.error = Some("DevTools has no profiling handle".to_owned()));
        return;
      };
      let mut started = false;
      state.update(|state| started = state.start(handle));
      if started {
        ticker.start();
      }
    })
    .id("devtools-profiler-record")
}

/// The board's Start recording: the toolbar's toggle with its mark in the destructive color, all of it in that
/// color on its muted fill while it records.
fn record_toggle() -> Recipe {
  changes::recording_toggle().base(TogglePart::Icon, |s| s.foreground(PaletteColor::Destructive))
}

/// One of the things a recording takes or leaves.
fn option(
  label: &str,
  icon: &str,
  on: bool,
  set: fn(&mut ProfilerState, bool),
  state: &Signal<ProfilerState>,
) -> Toggle {
  let state = state.clone();
  Toggle::new(label)
    .icon(icon)
    .size(ToggleSize::Xs)
    .rules(&kit::outlined_toggle())
    .on(on)
    .on_change(move |on| state.update(|state| set(state, on)))
    .id(format!(
      "devtools-profiler-{}",
      label.to_ascii_lowercase().replace(' ', "-")
    ))
}

fn button(label: &str, icon: &str) -> Button {
  Button::new(label)
    .variant(ButtonVariant::Outline)
    .size(ButtonSize::Xs)
    .icon(icon)
}

/// Set while a Load, Save or Export dialog of this process is open.
#[cfg(not(target_os = "ios"))]
static FILE_DIALOG_OPEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Load, and with a capture Save (a lurq recording file) and Export for the
/// Firefox Profiler. Each asks for its file on a thread of its own, so the
/// dialog does not hold up DevTools' passes.
#[cfg(not(target_os = "ios"))]
fn files(row: Row, state: &ProfilerState, signal: &Signal<ProfilerState>) -> Row {
  let on_thread = |run: fn(Signal<ProfilerState>)| {
    let signal = signal.clone();
    move |_| {
      // One file dialog at a time: a second press while one is open would
      // stack another over it.
      if FILE_DIALOG_OPEN.swap(true, std::sync::atomic::Ordering::AcqRel) {
        return;
      }
      let signal = signal.clone();
      std::thread::spawn(move || {
        run(signal);
        FILE_DIALOG_OPEN.store(false, std::sync::atomic::Ordering::Release);
      });
    }
  };
  let recording = state.recording();
  let mut row = row.child(
    // A file is not loaded over a recording that runs.
    button("Load", icons::FOLDER_OPEN)
      .disabled(recording)
      .on_click(on_thread(load_from_file))
      .id("devtools-profiler-load"),
  );
  if state.report.is_some() && !recording {
    row = row
      .child(
        button("Save", icons::DOWNLOAD)
          .on_click(on_thread(|signal| save_to_file(signal, false)))
          .id("devtools-profiler-save"),
      )
      .child(
        button("Export for Firefox Profiler", icons::EXTERNAL_LINK)
          .on_click(on_thread(|signal| save_to_file(signal, true)))
          .id("devtools-profiler-export"),
      );
  }
  row
}

/// iOS has no file dialog (`rfd` has no backend for it); the converters stay
/// available as `ProfileReport::to_recording_file` and `to_firefox_profile`.
#[cfg(target_os = "ios")]
fn files(row: Row, _state: &ProfilerState, _signal: &Signal<ProfilerState>) -> Row {
  row
}

#[cfg(not(target_os = "ios"))]
fn load_from_file(signal: Signal<ProfilerState>) {
  let Some(path) = rfd::FileDialog::new()
    .add_filter("lurq recording", &["json"])
    .pick_file()
  else {
    return;
  };
  // Read and parsed here, so a long file does not hold the state's lock.
  let report = std::fs::read_to_string(&path)
    .map_err(|error| format!("Load failed: {error}"))
    .and_then(|file| {
      crate::app::profiler::ProfileReport::from_recording_file(&file).map_err(|error| format!("Load failed: {error}"))
    });
  signal.update(|state| state.load(report));
}

/// Writes the capture as a lurq recording file, or for the Firefox Profiler.
#[cfg(not(target_os = "ios"))]
fn save_to_file(signal: Signal<ProfilerState>, firefox: bool) {
  let (filter, file_name) = if firefox {
    ("Firefox Profiler profile", "lurq-firefox-profile.json")
  } else {
    ("lurq recording", "recording.lurq.json")
  };
  let Some(path) = rfd::FileDialog::new()
    .add_filter(filter, &["json"])
    .set_file_name(file_name)
    .save_file()
  else {
    return;
  };
  let report = signal.with_untracked(|state| state.report.clone());
  let Some(report) = report else {
    return;
  };
  let file = if firefox {
    report.to_firefox_profile()
  } else {
    report.to_recording_file()
  };
  if let Err(error) = std::fs::write(&path, file) {
    signal.update(|state| state.error = Some(format!("Save failed: {error}")));
  }
}

/// How long a running recording has run, what it holds and how many samples it took.
pub(super) fn status_line(recording: &Recording) -> String {
  let Some(status) = recording.status else {
    return "recording".to_owned();
  };
  let mut line = format!(
    "{} · {} · {} samples",
    format_span(status.observed_ms - status.started_ms),
    format_bytes(status.recording.map_or(0, |recording| recording.size_bytes)),
    status.completed_samples
  );
  if let Some(screenshots) = status
    .recording
    .map(|recording| recording.screenshots)
    .filter(|count| *count > 0)
  {
    line.push_str(&format!(" · {screenshots} screenshots"));
  }
  if status.recording.is_some_and(|recording| recording.stopped_at_limit) {
    line.push_str(" · size limit reached");
  }
  line
}
