use lurq::{
  app::{
    component::Component,
    ctx::{CollisionStrategy, Ctx, Overlay, Placement},
    events::MouseEvent,
  },
  components::{Checkbox, Column, Row, TextInput, TextOverflow},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, HitTestBehavior, color::Color, dimension::Dimension},
  persistent_storage::{PersistentStorage, PersistentStorageSnapshotEntry, PersistentWrite},
};

use crate::style::{BG, BORDER, PRIMARY, SUCCESS, SURFACE, TEXT, TEXT_MUTED, WARNING, text};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CONTENT_PAD: f32 = 32.0;
const CARD_RADIUS: f32 = 8.0;

const PROFILE_KEY: &str = "demo.profile";
const OPEN_COUNT_KEY: &str = "demo.open_count";
const LAST_ACTION_KEY: &str = "demo.last_action";
const SAMPLE_SCORE_KEY: &str = "demo.sample_score";

#[derive(Clone, PartialEq, lurq::DevtoolsInspectable, lurq::PersistentValue)]
struct StoredProfile {
  display_name: String,
  notes: String,
  notifications_enabled: bool,
  save_count: u32,
}

impl Default for StoredProfile {
  fn default() -> Self {
    Self {
      display_name: "Ada Lovelace".to_owned(),
      notes: "Stored in target/demo-persistent-storage.redb".to_owned(),
      notifications_enabled: true,
      save_count: 0,
    }
  }
}

pub(crate) struct PersistentStorageDemo {
  storage: PersistentStorage,
  display_name: Signal<String>,
  notes: Signal<String>,
  notifications_enabled: Signal<bool>,
  save_count: Signal<u32>,
  open_count: Signal<u32>,
  status: Signal<String>,
  revision: Signal<u64>,
  active_type_tooltip: Signal<Option<String>>,
}

impl Component for PersistentStorageDemo {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let storage = ctx.app_ref().persistent_storage().clone();
    let profile = storage.value::<StoredProfile>(PROFILE_KEY).unwrap_or_default();
    let open_count = storage.value::<u32>(OPEN_COUNT_KEY).unwrap_or(0).saturating_add(1);

    let status = match storage.write_bulk([
      PersistentWrite::new(PROFILE_KEY, profile.clone()),
      PersistentWrite::new(OPEN_COUNT_KEY, open_count),
      PersistentWrite::new(LAST_ACTION_KEY, "opened persistent storage demo"),
      PersistentWrite::new(SAMPLE_SCORE_KEY, 98.5_f64),
    ]) {
      Ok(()) => "Loaded persisted values".to_owned(),
      Err(error) => format!("Storage error: {error}"),
    };

    Self {
      storage,
      display_name: ctx.signal(profile.display_name),
      notes: ctx.signal(profile.notes),
      notifications_enabled: ctx.signal(profile.notifications_enabled),
      save_count: ctx.signal(profile.save_count),
      open_count: ctx.signal(open_count),
      status: ctx.signal(status),
      revision: ctx.signal(ctx.app_ref().persistent_storage().revision()),
      active_type_tooltip: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let profile = StoredProfile {
      display_name: self.display_name.get(),
      notes: self.notes.get(),
      notifications_enabled: self.notifications_enabled.get(),
      save_count: self.save_count.get(),
    };
    let open_count = self.open_count.get();
    let status = self.status.get();
    let _revision = self.revision.get();
    let snapshot = self.storage.snapshot();

    Column::new()
      .spacing(24.0)
      .child(text("Persistent Storage", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
      .child(section_title("Profile"))
      .child(editor_card(
        profile.clone(),
        self.storage.clone(),
        self.display_name.clone(),
        self.notes.clone(),
        self.notifications_enabled.clone(),
        self.save_count.clone(),
        self.open_count.clone(),
        self.status.clone(),
        self.revision.clone(),
      ))
      .child(section_title("State"))
      .child(status_card(profile, open_count, &status))
      .child(section_title("Snapshot"))
      .child(snapshot_card(ctx, snapshot, self.active_type_tooltip.clone()))
      .padding(CONTENT_PAD)
      .width(FILL_WIDTH)
      .background(BG)
  }
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn card_frame() -> Column {
  Column::new()
    .width(FILL_WIDTH)
    .background(SURFACE)
    .rounded(CARD_RADIUS)
    .border_inside(1.0, Color::from_hex(BORDER))
}

fn editor_card(
  profile: StoredProfile,
  storage: PersistentStorage,
  display_name: Signal<String>,
  notes: Signal<String>,
  notifications_enabled: Signal<bool>,
  save_count: Signal<u32>,
  open_count: Signal<u32>,
  status: Signal<String>,
  revision: Signal<u64>,
) -> Element {
  let save_storage = storage.clone();
  let save_name = display_name.clone();
  let save_notes = notes.clone();
  let save_enabled = notifications_enabled.clone();
  let save_count_signal = save_count.clone();
  let save_status = status.clone();
  let save_revision = revision.clone();

  let reset_storage = storage.clone();
  let reset_name = display_name.clone();
  let reset_notes = notes.clone();
  let reset_enabled = notifications_enabled.clone();
  let reset_count = save_count.clone();
  let reset_open_count = open_count.clone();
  let reset_status = status.clone();
  let reset_revision = revision.clone();

  card_frame()
    .spacing(16.0)
    .child(field_stack(
      "Display name",
      text_input(display_name, "Display name").into(),
    ))
    .child(field_stack("Notes", notes_input(notes, "Notes").into()))
    .child(checkbox_row(
      "Notifications",
      notifications_enabled,
      profile.notifications_enabled,
    ))
    .child(
      Row::new()
        .spacing(12.0)
        .align_items(Alignment::Center)
        .child(button("Save", PRIMARY, move |_| {
          let next_count = save_count_signal.get().saturating_add(1);
          let next_profile = StoredProfile {
            display_name: save_name.get(),
            notes: save_notes.get(),
            notifications_enabled: save_enabled.get(),
            save_count: next_count,
          };
          match save_storage.write_bulk([
            PersistentWrite::new(PROFILE_KEY, next_profile),
            PersistentWrite::new(LAST_ACTION_KEY, "saved profile"),
          ]) {
            Ok(()) => {
              save_count_signal.set(next_count);
              save_status.set(format!("Saved profile #{next_count}"));
              save_revision.set(save_storage.revision());
            }
            Err(error) => save_status.set(format!("Storage error: {error}")),
          }
        }))
        .child(button("Seed bulk values", SUCCESS, {
          let storage = storage.clone();
          let status = status.clone();
          let revision = revision.clone();
          move |_| match storage.write_bulk([
            PersistentWrite::new("demo.bulk.user", "bulk-user"),
            PersistentWrite::new("demo.bulk.enabled", true),
            PersistentWrite::new("demo.bulk.count", 3_u32),
            PersistentWrite::new(LAST_ACTION_KEY, "seeded bulk values"),
          ]) {
            Ok(()) => {
              status.set("Seeded bulk values".to_owned());
              revision.set(storage.revision());
            }
            Err(error) => status.set(format!("Storage error: {error}")),
          }
        }))
        .child(button("Reset", WARNING, move |_| {
          let profile = StoredProfile::default();
          let result = reset_storage.write_bulk([
            PersistentWrite::new(PROFILE_KEY, profile.clone()),
            PersistentWrite::new(OPEN_COUNT_KEY, 0_u32),
            PersistentWrite::new(LAST_ACTION_KEY, "reset demo values"),
            PersistentWrite::new(SAMPLE_SCORE_KEY, 98.5_f64),
          ]);
          for key in ["demo.bulk.user", "demo.bulk.enabled", "demo.bulk.count"] {
            let _ = reset_storage.remove_value(key);
          }
          match result {
            Ok(()) => {
              reset_name.set(profile.display_name);
              reset_notes.set(profile.notes);
              reset_enabled.set(profile.notifications_enabled);
              reset_count.set(profile.save_count);
              reset_open_count.set(0);
              reset_status.set("Reset demo values".to_owned());
              reset_revision.set(reset_storage.revision());
            }
            Err(error) => reset_status.set(format!("Storage error: {error}")),
          }
        }))
        .width(FILL_WIDTH),
    )
    .padding(24.0)
    .into()
}

fn status_card(profile: StoredProfile, open_count: u32, status: &str) -> Element {
  card_frame()
    .spacing(12.0)
    .child(
      Row::new()
        .spacing(12.0)
        .child(metric("opens", &open_count.to_string()))
        .child(metric("saves", &profile.save_count.to_string()))
        .child(metric(
          "notifications",
          if profile.notifications_enabled { "on" } else { "off" },
        ))
        .width(FILL_WIDTH),
    )
    .child(kv_row("profile", &profile.display_name))
    .child(kv_row("last status", status))
    .padding(24.0)
    .into()
}

fn snapshot_card(
  ctx: &mut Ctx,
  snapshot: Result<Vec<PersistentStorageSnapshotEntry>, impl std::fmt::Display>,
  active_type_tooltip: Signal<Option<String>>,
) -> Element {
  let mut card = card_frame().spacing(8.0).child(snapshot_header()).padding(16.0);

  match snapshot {
    Ok(entries) if entries.is_empty() => {
      card = card.child(text("No persistent values", 13.0, FontWeight::Medium, TEXT_MUTED));
    }
    Ok(entries) => {
      for entry in entries {
        card = card.child(snapshot_row(ctx, entry, active_type_tooltip.clone()));
      }
    }
    Err(error) => {
      card = card.child(text(
        &format!("Storage error: {error}"),
        13.0,
        FontWeight::Medium,
        WARNING,
      ));
    }
  }

  card.into()
}

fn snapshot_header() -> Element {
  Row::new()
    .spacing(8.0)
    .align_items(Alignment::Center)
    .child(single_line_cell("Key", 2.2, FontWeight::Bold, TEXT_MUTED))
    .child(single_line_cell("Type", 1.0, FontWeight::Bold, TEXT_MUTED))
    .child(single_line_cell("Value", 2.6, FontWeight::Bold, TEXT_MUTED))
    .child(single_line_cell("Bytes", 0.6, FontWeight::Bold, TEXT_MUTED))
    .width(FILL_WIDTH)
    .height(24.0)
    .into()
}

fn snapshot_row(
  ctx: &mut Ctx,
  entry: PersistentStorageSnapshotEntry,
  active_type_tooltip: Signal<Option<String>>,
) -> Element {
  Row::new()
    .spacing(8.0)
    .align_items(Alignment::Center)
    .child(single_line_cell(&entry.key, 2.2, FontWeight::Medium, TEXT))
    .child(type_cell(ctx, &entry, active_type_tooltip))
    .child(wrapping_cell(&entry.value, 2.6, FontWeight::Medium, TEXT))
    .child(single_line_cell(
      &entry.byte_len.to_string(),
      0.6,
      FontWeight::Medium,
      TEXT_MUTED,
    ))
    .width(FILL_WIDTH)
    .min_height(28.0)
    .padding_vertical(4.0)
    .border_inside(1.0, Color::from_hex("#263449"))
    .rounded(5.0)
    .into()
}

fn type_cell(
  ctx: &mut Ctx,
  entry: &PersistentStorageSnapshotEntry,
  active_type_tooltip: Signal<Option<String>>,
) -> Element {
  let anchor = ctx.element_ref();
  let show_tooltip = entry.full_type_name != entry.type_name;
  let should_log = entry.key == PROFILE_KEY;
  let tooltip_open = active_type_tooltip.get().as_deref() == Some(entry.full_type_name.as_str());
  let full_type_name = entry.full_type_name.clone();
  if show_tooltip && should_log {
    log_tooltip("demo", "render", &entry.type_name, &entry.full_type_name, tooltip_open);
  }
  let mut cell = Row::new()
    .align_items(Alignment::Center)
    .child(
      text(&entry.type_name, 12.0, FontWeight::Medium, TEXT_MUTED)
        .flex(1.0)
        .height(20.0)
        .nowrap()
        .text_overflow(TextOverflow::Elipsis),
    )
    .ref_element(anchor.clone())
    .flex(1.0)
    .min_height(22.0)
    .hovered(|style| style.background("#273449"));

  if show_tooltip {
    cell = cell
      .on_mouse_enter({
        let active_type_tooltip = active_type_tooltip.clone();
        let full_type_name = full_type_name.clone();
        let type_name = entry.type_name.clone();
        let should_log = should_log;
        move || {
          if should_log {
            log_tooltip("demo", "enter", &type_name, &full_type_name, true);
          }
          active_type_tooltip.set(Some(full_type_name.clone()));
        }
      })
      .on_mouse_leave({
        let active_type_tooltip = active_type_tooltip.clone();
        let full_type_name = full_type_name.clone();
        let type_name = entry.type_name.clone();
        let should_log = should_log;
        move || {
          if should_log {
            log_tooltip("demo", "leave", &type_name, &full_type_name, false);
          }
          active_type_tooltip.set(None);
        }
      })
      .child(
        Overlay::new(type_tooltip(&entry.full_type_name))
          .anchor(anchor.clone())
          .open_when(tooltip_open)
          .placement(Placement::TopStart)
          .offset(0.0, 6.0)
          .collision(CollisionStrategy::FlipThenClamp)
          .hit_test(HitTestBehavior::ContentOnly),
      );
  }

  cell.into()
}

fn log_tooltip(scope: &str, event: &str, type_name: &str, full_type_name: &str, open: bool) {
  tracing::info!(
    target: "lurq::persistent_storage_tooltip",
    scope,
    event,
    type_name,
    full_type_name,
    open,
    "persistent storage type tooltip"
  );
  eprintln!(
    "[persistent-storage-tooltip] scope={scope} event={event} type={type_name} full={full_type_name} open={open}"
  );
}

fn type_tooltip(full_type_name: &str) -> Element {
  Row::new()
    .key("persistent-storage-type-tooltip")
    .child(
      text(full_type_name, 11.0, FontWeight::Medium, TEXT)
        .nowrap()
        .text_overflow(TextOverflow::Elipsis),
    )
    .max_width(560.0)
    .padding_horizontal(10.0)
    .padding_vertical(7.0)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(6.0)
    .into()
}

fn single_line_cell(content: &str, flex: f32, weight: FontWeight, color: &str) -> Element {
  text(content, 12.0, weight, color)
    .height(20.0)
    .flex(flex)
    .nowrap()
    .text_overflow(TextOverflow::Elipsis)
    .into()
}

fn wrapping_cell(content: &str, flex: f32, weight: FontWeight, color: &str) -> Element {
  text(content, 12.0, weight, color).flex(flex).into()
}

fn field_stack(label: &str, input: Element) -> Column {
  Column::new()
    .spacing(6.0)
    .child(text(label, 12.0, FontWeight::Bold, TEXT_MUTED))
    .child(input)
    .width(FILL_WIDTH)
}

fn text_input(value: Signal<String>, placeholder: &str) -> TextInput {
  TextInput::new(value)
    .placeholder(placeholder)
    .single_line()
    .width(FILL_WIDTH)
    .height(38.0)
    .padding_horizontal(12.0)
    .padding_vertical(6.0)
    .background("#ffffff")
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(6.0)
    .cursor(CursorIcon::Text)
    .focused(|style| style.border_inside(2.0, Color::from_hex(PRIMARY)))
}

fn notes_input(value: Signal<String>, placeholder: &str) -> TextInput {
  TextInput::new(value)
    .placeholder(placeholder)
    .rows(2, 4)
    .width(FILL_WIDTH)
    .padding_horizontal(12.0)
    .padding_vertical(8.0)
    .background("#ffffff")
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(6.0)
    .cursor(CursorIcon::Text)
    .focused(|style| style.border_inside(2.0, Color::from_hex(PRIMARY)))
}

fn checkbox_row(label: &str, value: Signal<bool>, enabled: bool) -> Element {
  Row::new()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(
      Checkbox::new(value)
        .size(20.0, 20.0)
        .background(if enabled { SUCCESS } else { "#ffffff" })
        .border_inside(1.0, Color::from_hex(BORDER))
        .rounded(4.0)
        .cursor(CursorIcon::Pointer)
        .focused(|style| style.border_inside(2.0, Color::from_hex(PRIMARY))),
    )
    .child(text(label, 14.0, FontWeight::Bold, TEXT))
    .height(36.0)
    .width(FILL_WIDTH)
    .into()
}

fn button(label: &str, fill: &'static str, handler: impl Fn(MouseEvent) + Send + Sync + 'static) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 12.0, FontWeight::Bold, "#ffffff"))
    .height(34.0)
    .padding_horizontal(14.0)
    .background(fill)
    .rounded(6.0)
    .cursor(CursorIcon::Pointer)
    .hovered(|style| style.background("#60a5fa"))
    .active(|style| style.background("#2563eb"))
    .on_click(handler)
    .into()
}

fn metric(label: &str, value: &str) -> Element {
  Column::new()
    .spacing(3.0)
    .child(text(label, 11.0, FontWeight::Bold, TEXT_MUTED))
    .child(text(value, 18.0, FontWeight::Bold, TEXT))
    .padding(12.0)
    .background("#101827")
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(6.0)
    .flex(1.0)
    .into()
}

fn kv_row(label: &str, value: &str) -> Element {
  Row::new()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(text(label, 12.0, FontWeight::Bold, TEXT_MUTED).width(96.0))
    .child(text(value, 13.0, FontWeight::Medium, TEXT).flex(1.0))
    .width(FILL_WIDTH)
    .into()
}
