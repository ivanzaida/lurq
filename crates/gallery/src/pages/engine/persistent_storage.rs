//! The page of the persistent store: an app's small typed values, kept in a file between its runs. Its playground
//! writes, reads and removes a value of a key, as a type; its sections are the demo's Persistent Storage page: a
//! profile saved as one value, the state of what was saved, and a snapshot of everything the store holds.
//!
//! The store is a file of a desktop app (the `persistent_storage` feature, which the browser's build is without):
//! there the page says so in a line.

#[cfg(target_arch = "wasm32")]
pub use browser::PersistentStoragePage;
#[cfg(not(target_arch = "wasm32"))]
pub use desktop::PersistentStoragePage;

/// What the page is in a browser: a line in place of the store.
#[cfg(target_arch = "wasm32")]
mod browser {
  use lurq::{
    app::{component::Component, ctx::Ctx},
    node::Element,
  };

  use crate::{
    doc::{Frame, Header, Section},
    kit::{FILL, quiet},
  };

  pub struct PersistentStoragePage {
    frame: Frame,
  }

  impl Component for PersistentStoragePage {
    type Props = ();

    fn create(ctx: &mut Ctx) -> Self {
      Self { frame: Frame::new(ctx) }
    }

    fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
      self.frame.page(
        ctx,
        Header {
          breadcrumb: &["Engine", "Persistent Storage"],
          title: "Persistent Storage",
          description: "Small typed values an app keeps between its runs: preferences, counters, the tab that was \
                        open.",
          import: Some("use lurq::persistent_storage::{PersistentStorage, PersistentWrite};"),
          status: Some("Stable"),
          since: None,
          source: file!(),
        },
        vec![Section::new(
          "In a browser",
          "The store is a file of a desktop app, and this build of the gallery is a page of a browser.",
          quiet(
            "The persistent store is a file of a desktop app: open the gallery on a desktop to try it.",
            13.0,
          )
          .width(FILL),
        )],
      )
    }
  }
}

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
  use std::{path::PathBuf, sync::Arc};

  use lurq::{
    app::{
      component::Component,
      ctx::Ctx,
      events::MouseEvent,
      theme::{PaletteColor, RadiusSize},
    },
    components::{Column, Row, TextOverflow},
    core::Signal,
    layout::{Alignment, text_style::FontWeight},
    node::Element,
    persistent_storage::{PersistentStorage, PersistentStorageSnapshotEntry, PersistentWrite},
    ui::{Button, ButtonSize, ButtonVariant, Field, Input, Switch, Textarea, Tooltip},
  };

  use super::super::kit::{named, well};
  use crate::{
    doc::{Frame, Header, Section},
    kit::{FILL, fit, line, mono, props_table, quiet, text},
    playground::{Playground, Shown, Spec, divider, field, prop},
  };

  const PROFILE_KEY: &str = "engine.storage.profile";
  const OPEN_COUNT_KEY: &str = "engine.storage.open_count";
  const LAST_ACTION_KEY: &str = "engine.storage.last_action";
  const SAMPLE_SCORE_KEY: &str = "engine.storage.sample_score";
  const BULK_KEYS: [&str; 3] = [
    "engine.storage.bulk.user",
    "engine.storage.bulk.enabled",
    "engine.storage.bulk.count",
  ];

  /// The keys of the playground, and the types it reads and writes them as.
  const KEYS: [(&str, usize); 3] = [
    ("engine.storage.note", 0),
    ("engine.storage.count", 1),
    ("engine.storage.flag", 2),
  ];
  const TYPES: [(&str, usize); 3] = [("String", 0), ("u32", 1), ("bool", 2)];
  /// The least width of the snapshot's table: its four columns, the value wide enough to read.
  const SNAPSHOT_WIDTH: f32 = 560.0;

  /// The profile the page saves, one value of the store: a struct whose fields are values of the store too.
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
        notes: "Kept in the gallery's store".to_owned(),
        notifications_enabled: true,
        save_count: 0,
      }
    }
  }

  /// Where the app's store keeps its values, as far as the page can tell. The store does not say: the gallery
  /// opens its file at its start (`main.rs`), and a store that holds a file locks it for itself. The file is the
  /// app's where it is locked so; where it is not there, or not locked, the store is in memory (a test, which
  /// sets no file). A second gallery whose store found the file held by the first is told wrong: it is in memory.
  fn kept() -> Option<PathBuf> {
    let path = crate::cache_folder().join("storage.redb");
    // Asked with a lock that reads and is let go at once: nothing is written to the file.
    let file = std::fs::File::open(&path).ok()?;
    match file.try_lock_shared() {
      Err(std::fs::TryLockError::WouldBlock) => Some(path),
      _ => None,
    }
  }

  /// A small button of the page's own.
  fn small(label: &str) -> Button {
    Button::new(label).variant(ButtonVariant::Outline).size(ButtonSize::Sm)
  }

  /// What a read of a key as a type gave, written as the code would print it.
  fn read(storage: &PersistentStorage, key: &str, kind: usize) -> String {
    match kind {
      0 => format!("{:?}", storage.value::<String>(key)),
      1 => format!("{:?}", storage.value::<u32>(key)),
      _ => format!("{:?}", storage.value::<bool>(key)),
    }
  }

  /// Writes `value` to `key` as the type `kind` names: what was written, or why nothing was.
  fn write(storage: &PersistentStorage, key: &str, kind: usize, value: &str) -> Result<String, String> {
    let done = match kind {
      0 => storage.set_value(key, value),
      1 => match value.trim().parse::<u32>() {
        Ok(number) => storage.set_value(key, number),
        Err(_) => return Err(format!("{value:?} is not a u32")),
      },
      _ => match value.trim().parse::<bool>() {
        Ok(flag) => storage.set_value(key, flag),
        Err(_) => return Err(format!("{value:?} is not a bool")),
      },
    };
    done
      .map(|()| format!("{value:?}"))
      .map_err(|error| format!("Storage error: {error}"))
  }

  pub struct PersistentStoragePage {
    frame: Frame,
    playground: Playground,
    storage: PersistentStorage,
    /// The file the store is in, where the page could tell it is in one.
    file: Option<PathBuf>,
    /// The store's revision as the page last read it: a write sets it, and the page renders what is stored again.
    revision: Signal<u64>,
    /// The playground: an index into [`KEYS`] and one into [`TYPES`], the value typed, and what the last read
    /// gave.
    key: Signal<usize>,
    kind: Signal<usize>,
    value: Signal<String>,
    last_read: Signal<String>,
    display_name: Signal<String>,
    notes: Signal<String>,
    notifications_enabled: Signal<bool>,
    save_count: Signal<u32>,
    open_count: Signal<u32>,
    status: Signal<String>,
  }

  impl PersistentStoragePage {
    fn play(&self, ctx: &mut Ctx) -> Element {
      let (key_at, kind, value) = (self.key.get(), self.kind.get(), self.value.get());
      let key = KEYS[key_at.min(KEYS.len() - 1)].0;
      let kind_name = TYPES[kind.min(TYPES.len() - 1)].0;
      let _ = self.revision.get();
      let stored = self
        .storage
        .snapshot()
        .ok()
        .and_then(|entries| entries.into_iter().find(|entry| entry.key == key));
      let last_read = self.last_read.get();
      let (storage, record, revision, read_back) = (
        self.storage.clone(),
        self.playground.recorder(),
        self.revision.clone(),
        self.last_read.clone(),
      );
      let build = {
        let (value, stored) = (value.clone(), stored.clone());
        move || -> Element {
          let (to_write, to_read, to_remove) = (
            (storage.clone(), record.clone(), revision.clone(), value.clone()),
            (storage.clone(), record.clone(), read_back.clone()),
            (storage.clone(), record.clone(), revision.clone()),
          );
          let buttons = Row::new()
            .spacing(8.0)
            .wrap()
            .child(
              Button::new("Write")
                .size(ButtonSize::Sm)
                .on_click(move |_: MouseEvent| {
                  let (storage, record, revision, value) = &to_write;
                  match write(storage, key, kind, value) {
                    Ok(written) => {
                      record(format!("set_value({key:?}, {written}) = Ok"));
                      revision.set(storage.revision());
                    }
                    Err(why) => record(why),
                  }
                }),
            )
            .child(small("Read").on_click(move |_: MouseEvent| {
              let (storage, record, read_back) = &to_read;
              let got = read(storage, key, kind);
              record(format!("value::<{kind_name}>({key:?}) = {got}"));
              read_back.set(got);
            }))
            .child(small("Remove").on_click(move |_: MouseEvent| {
              let (storage, record, revision) = &to_remove;
              match storage.remove_value(key) {
                Ok(()) => {
                  record(format!("remove_value({key:?}) = Ok"));
                  revision.set(storage.revision());
                }
                Err(error) => record(format!("Storage error: {error}")),
              }
            }));
          let held = match &stored {
            Some(entry) => format!("{}: {}, {} bytes", entry.type_name, entry.value, entry.byte_len),
            None => "nothing".to_owned(),
          };
          Column::new()
            .width(320.0)
            .max_width(FILL)
            .spacing(12.0)
            .child(buttons)
            .child(named("stored", mono(&held, 12.0).width(FILL)))
            .child(named(
              "last read",
              mono(if last_read.is_empty() { "-" } else { &last_read }, 12.0).width(FILL),
            ))
            .into()
        }
      };

      let code = vec![
        "let storage = ctx.app_ref().persistent_storage();".to_owned(),
        match kind {
          0 => format!("storage.set_value({key:?}, {value:?})?;"),
          _ => format!("storage.set_value({key:?}, {}_{kind_name})?;", value.trim()),
        },
        format!("let read: Option<{kind_name}> = storage.value::<{kind_name}>({key:?});"),
        format!("storage.remove_value({key:?})?;"),
      ];
      let tree = vec![
        format!("column   {}", self.playground.measured()),
        "├─ row   Write, Read, Remove".to_owned(),
        "├─ stored".to_owned(),
        "└─ last read".to_owned(),
      ];
      let (set_key, set_kind) = (self.key.clone(), self.kind.clone());
      let reset = {
        let (key, kind, value, last_read) = (
          self.key.clone(),
          self.kind.clone(),
          self.value.clone(),
          self.last_read.clone(),
        );
        Arc::new(move || {
          key.set(0);
          kind.set(0);
          value.set("Hello".to_owned());
          last_read.set(String::new());
        })
      };
      self.playground.card(
        ctx,
        Spec {
          title: "Persistent value",
          make: Arc::new(build.clone()),
          shown: Shown::built(move |_| build()),
          fill: false,
          forceable: false,
          facts: if stored.is_some() {
            "stored".to_owned()
          } else {
            "not stored".to_owned()
          },
          event: "set_value",
          code,
          tree,
          controls: vec![
            prop(
              "key",
              "&str",
              self
                .playground
                .select(0, &KEYS, &key_at, move |value| set_key.set(value)),
            ),
            prop(
              "as",
              "T: PersistentValue",
              self
                .playground
                .select(1, &TYPES, &kind, move |value| set_kind.set(value)),
            ),
            prop("value", "T", field(&self.value)),
            divider(),
          ],
          changed: [key_at != 0, kind != 0, value != "Hello"]
            .into_iter()
            .filter(|changed| *changed)
            .count(),
          reset,
        },
      )
    }

    /// The fields of the profile and what saves, seeds and resets it.
    fn editor(&self) -> Column {
      let save = {
        let (storage, status, revision) = (self.storage.clone(), self.status.clone(), self.revision.clone());
        let (name, notes, enabled, count) = (
          self.display_name.clone(),
          self.notes.clone(),
          self.notifications_enabled.clone(),
          self.save_count.clone(),
        );
        move |_: MouseEvent| {
          let next = count.get().saturating_add(1);
          let profile = StoredProfile {
            display_name: name.get(),
            notes: notes.get(),
            notifications_enabled: enabled.get(),
            save_count: next,
          };
          match storage.write_bulk([
            PersistentWrite::new(PROFILE_KEY, profile),
            PersistentWrite::new(LAST_ACTION_KEY, "saved profile"),
          ]) {
            Ok(()) => {
              count.set(next);
              status.set(format!("Saved profile #{next}"));
              revision.set(storage.revision());
            }
            Err(error) => status.set(format!("Storage error: {error}")),
          }
        }
      };
      let seed = {
        let (storage, status, revision) = (self.storage.clone(), self.status.clone(), self.revision.clone());
        move |_: MouseEvent| match storage.write_bulk([
          PersistentWrite::new(BULK_KEYS[0], "bulk-user"),
          PersistentWrite::new(BULK_KEYS[1], true),
          PersistentWrite::new(BULK_KEYS[2], 3_u32),
          PersistentWrite::new(LAST_ACTION_KEY, "seeded bulk values"),
        ]) {
          Ok(()) => {
            status.set("Seeded bulk values".to_owned());
            revision.set(storage.revision());
          }
          Err(error) => status.set(format!("Storage error: {error}")),
        }
      };
      let reset = {
        let (storage, status, revision) = (self.storage.clone(), self.status.clone(), self.revision.clone());
        let (name, notes, enabled, count, opens) = (
          self.display_name.clone(),
          self.notes.clone(),
          self.notifications_enabled.clone(),
          self.save_count.clone(),
          self.open_count.clone(),
        );
        move |_: MouseEvent| {
          let profile = StoredProfile::default();
          let result = storage.write_bulk([
            PersistentWrite::new(PROFILE_KEY, profile.clone()),
            PersistentWrite::new(OPEN_COUNT_KEY, 0_u32),
            PersistentWrite::new(LAST_ACTION_KEY, "reset the page's values"),
            PersistentWrite::new(SAMPLE_SCORE_KEY, 98.5_f64),
          ]);
          for key in BULK_KEYS {
            let _ = storage.remove_value(key);
          }
          match result {
            Ok(()) => {
              name.set(profile.display_name);
              notes.set(profile.notes);
              enabled.set(profile.notifications_enabled);
              count.set(profile.save_count);
              opens.set(0);
              status.set("Reset the page's values".to_owned());
            }
            Err(error) => status.set(format!("Storage error: {error}")),
          }
          revision.set(storage.revision());
        }
      };
      Column::new()
        .width(FILL)
        .spacing(16.0)
        .child(Field::new(
          "Display name",
          Input::new(self.display_name.clone()).full_width(true),
        ))
        .child(Field::new(
          "Notes",
          Element::from(Textarea::new(self.notes.clone()).rows(2, 4).full_width(true)),
        ))
        .child(Switch::new("Notifications").bind(self.notifications_enabled.clone()))
        .child(
          Row::new()
            .width(FILL)
            .spacing(8.0)
            .wrap()
            .child(Button::new("Save").size(ButtonSize::Sm).on_click(save))
            .child(small("Seed bulk values").on_click(seed))
            .child(
              Button::new("Reset")
                .variant(ButtonVariant::Destructive)
                .size(ButtonSize::Sm)
                .on_click(reset),
            ),
        )
    }

    /// What was saved: the opens, the saves and whether notifications are on, the name and the last status.
    fn state(&self) -> Column {
      let metric = |label: &str, value: &str| {
        Column::new()
          .width(120.0)
          .max_width(FILL)
          .spacing(3.0)
          .padding(12.0)
          .rounded(RadiusSize::Md)
          .background(PaletteColor::Muted)
          .child(quiet(label, 11.0))
          .child(text(value, 18.0, FontWeight::Bold))
      };
      let status = self.status.get();
      let where_kept = match &self.file {
        Some(path) => format!("in {}", path.display()),
        None => "in memory: no file was set, and it is gone when the gallery closes".to_owned(),
      };
      Column::new()
        .width(FILL)
        .spacing(12.0)
        .child(
          Row::new()
            .width(FILL)
            .spacing(12.0)
            .wrap()
            .child(metric("opens", &self.open_count.get().to_string()))
            .child(metric("saves", &self.save_count.get().to_string()))
            .child(metric(
              "notifications",
              if self.notifications_enabled.get() { "on" } else { "off" },
            )),
        )
        .child(named(
          "profile",
          text(&self.display_name.get(), 13.0, FontWeight::Medium).width(FILL),
        ))
        .child(named(
          "last status",
          text(&status, 13.0, FontWeight::Medium).width(FILL),
        ))
        .child(named("kept", mono(&where_kept, 12.0).width(FILL)))
    }

    /// Everything the store holds, a row a key: its type, with the full name of it under the pointer, its
    /// value and its size.
    fn snapshot(&self, ctx: &mut Ctx) -> Element {
      let _ = self.revision.get();
      let cell = |content: &str, share: f32, weight: FontWeight| {
        text(content, 12.0, weight)
          .flex(share)
          .min_width(0.0)
          .nowrap()
          .text_overflow(TextOverflow::Elipsis)
      };
      let row = || {
        Row::new()
          .width(FILL)
          .min_height(28.0)
          .padding_horizontal(8.0)
          .padding_vertical(4.0)
          .spacing(8.0)
          .align_items(Alignment::Center)
      };
      let mut table = well().padding(8.0).spacing(2.0).child(
        row()
          .child(cell("Key", 2.2, FontWeight::SemiBold).color(PaletteColor::MutedForeground))
          .child(cell("Type", 1.0, FontWeight::SemiBold).color(PaletteColor::MutedForeground))
          .child(cell("Value", 2.6, FontWeight::SemiBold).color(PaletteColor::MutedForeground))
          .child(cell("Bytes", 0.6, FontWeight::SemiBold).color(PaletteColor::MutedForeground)),
      );
      match self.storage.snapshot() {
        Ok(entries) if entries.is_empty() => {
          table = table.child(quiet("No persistent values", 13.0));
        }
        Ok(entries) => {
          for entry in entries {
            table = table.child(Self::entry(ctx, row(), entry));
          }
        }
        Err(error) => {
          table = table
            .child(text(&format!("Storage error: {error}"), 13.0, FontWeight::Medium).color(PaletteColor::Destructive));
        }
      }
      fit("storage-snapshot", self.frame.stage_room(), SNAPSHOT_WIDTH, table)
    }

    /// A row of the snapshot: the type's full name is under the pointer where the short one is not all of it.
    fn entry(ctx: &mut Ctx, row: Row, entry: PersistentStorageSnapshotEntry) -> Row {
      let short = text(&entry.type_name, 12.0, FontWeight::Medium)
        .color(PaletteColor::MutedForeground)
        .nowrap()
        .text_overflow(TextOverflow::Elipsis);
      let kind = Row::new().flex(1.0).min_width(0.0).child(short);
      let kind: Element = if entry.full_type_name != entry.type_name {
        Row::new()
          .flex(1.0)
          .min_width(0.0)
          .child(Tooltip::new(&entry.full_type_name).mount(ctx, kind.width(FILL)))
          .into()
      } else {
        kind.into()
      };
      row
        .border_top(line())
        .child(
          text(&entry.key, 12.0, FontWeight::Medium)
            .flex(2.2)
            .min_width(0.0)
            .nowrap()
            .text_overflow(TextOverflow::Elipsis),
        )
        .child(kind)
        .child(mono(&entry.value, 12.0).flex(2.6).min_width(0.0))
        .child(
          mono(&entry.byte_len.to_string(), 12.0)
            .color(PaletteColor::MutedForeground)
            .flex(0.6)
            .min_width(0.0),
        )
    }
  }

  impl Component for PersistentStoragePage {
    type Props = ();

    fn create(ctx: &mut Ctx) -> Self {
      let storage = ctx.app_ref().persistent_storage();
      let profile = storage.value::<StoredProfile>(PROFILE_KEY).unwrap_or_default();
      let open_count = storage.value::<u32>(OPEN_COUNT_KEY).unwrap_or(0).saturating_add(1);
      // Each time the page is opened it counts so, and writes what it shows if it is not there yet.
      let status = match storage.write_bulk([
        PersistentWrite::new(PROFILE_KEY, profile.clone()),
        PersistentWrite::new(OPEN_COUNT_KEY, open_count),
        PersistentWrite::new(LAST_ACTION_KEY, "opened the page"),
        PersistentWrite::new(SAMPLE_SCORE_KEY, 98.5_f64),
      ]) {
        Ok(()) => "Loaded persisted values".to_owned(),
        Err(error) => format!("Storage error: {error}"),
      };
      Self {
        frame: Frame::new(ctx),
        playground: Playground::new(ctx, 2),
        file: kept(),
        revision: ctx.signal(storage.revision()),
        storage,
        key: ctx.signal(0),
        kind: ctx.signal(0),
        value: ctx.signal("Hello".to_owned()),
        last_read: ctx.signal(String::new()),
        display_name: ctx.signal(profile.display_name),
        notes: ctx.signal(profile.notes),
        notifications_enabled: ctx.signal(profile.notifications_enabled),
        save_count: ctx.signal(profile.save_count),
        open_count: ctx.signal(open_count),
        status: ctx.signal(status),
      }
    }

    fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
      let rows = [
        (
          "set_persistent_storage_path",
          "App, impl Into<PathBuf>",
          "",
          "Opens the file the store keeps its values in, before the app is mounted. Without it the store is \
           in memory.",
        ),
        (
          "persistent_storage",
          "AppRef -> PersistentStorage",
          "",
          "The app's store, shared: ctx.app_ref().persistent_storage().",
        ),
        (
          "value",
          "&str -> Option<T>",
          "",
          "The value of a key, as T. None where there is none, or where it was stored as another type.",
        ),
        (
          "set_value",
          "&str, T",
          "",
          "Stores a value: a number, a bool, a char, a string, or a struct that derives PersistentValue.",
        ),
        (
          "write_bulk",
          "PersistentWrite...",
          "",
          "Stores several values in one transaction. PersistentWrite::new gives values of other types one type.",
        ),
        ("remove_value", "&str", "", "Takes a key out of the store."),
        (
          "snapshot",
          "Vec<PersistentStorageSnapshotEntry>",
          "",
          "Every key with its type, its value as it is written, and its size: what a tool shows.",
        ),
        (
          "revision",
          "u64",
          "",
          "Goes up at each write. The store tells no component of a change: read it again after one.",
        ),
      ];
      let playground = self.play(ctx);
      let snapshot = self.snapshot(ctx);
      self.frame.page(
        ctx,
        Header {
          breadcrumb: &["Engine", "Persistent Storage"],
          title: "Persistent Storage",
          description: "Small typed values an app keeps between its runs: preferences, counters, the tab that \
                        was open. The gallery keeps its store in a file of its own, so what this page saves is \
                        here the next time it is opened.",
          import: Some("use lurq::persistent_storage::{PersistentStorage, PersistentWrite};"),
          status: Some("Stable"),
          since: None,
          source: file!(),
        },
        vec![
          Section::new(
            "Playground",
            "Write a value to a key as a type, read it back, remove it. A key stored as one type and read as \
             another reads None. What was done is told in the Events tab.",
            playground,
          ),
          Section::new(
            "Profile",
            "A profile saved as one value, a struct that derives PersistentValue. Save writes it with the last \
             action in one transaction; Seed writes three values of other types; Reset puts the page's values \
             back and removes the seeded ones.",
            self.frame.demo(
              0,
              self.editor(),
              "storage.write_bulk([PersistentWrite::new(key, profile), ..])",
              &[
                "#[derive(Clone, PartialEq, lurq::PersistentValue)]",
                "struct StoredProfile { display_name: String, notes: String, notifications_enabled: bool, \
                 save_count: u32 }",
                "",
                "let storage = ctx.app_ref().persistent_storage();",
                "storage.write_bulk([",
                "    PersistentWrite::new(\"engine.storage.profile\", profile),",
                "    PersistentWrite::new(\"engine.storage.last_action\", \"saved profile\"),",
                "])?;",
                "",
                "let profile = storage.value::<StoredProfile>(\"engine.storage.profile\").unwrap_or_default();",
              ],
            ),
          ),
          Section::new(
            "State",
            "What the store holds for the page: how many times it was opened, how many times the profile was \
             saved, and what the last write said. An error of the store is said here too.",
            self.frame.demo(
              1,
              self.state(),
              "storage.value::<u32>(\"engine.storage.open_count\")",
              &[
                "let opens = storage.value::<u32>(\"engine.storage.open_count\").unwrap_or(0) + 1;",
                "match storage.set_value(\"engine.storage.open_count\", opens) {",
                "    Ok(()) => status.set(\"Loaded persisted values\".into()),",
                "    Err(error) => status.set(format!(\"Storage error: {error}\")),",
                "}",
              ],
            ),
          ),
          Section::new(
            "Snapshot",
            "Everything the gallery's store holds, the other pages' values too. A type whose full name is longer \
             than its short one shows it under the pointer.",
            self.frame.demo(
              2,
              snapshot,
              "storage.snapshot()",
              &[
                "for entry in storage.snapshot()? {",
                "    println!(\"{} {} {} {}\", entry.key, entry.type_name, entry.value, entry.byte_len);",
                "}",
              ],
            ),
          ),
          Section::new(
            "Props",
            "What the store is opened with, and what it does.",
            props_table(&rows, &[], self.frame.room()),
          ),
        ],
      )
    }
  }
}
