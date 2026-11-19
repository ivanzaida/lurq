//! The Storage tab: what the inspected app keeps in its persistent storage, a row for each key with the type of
//! its value, the value as it is written and how many bytes it takes. Built from the board
//! (`pencil/lurq-devtools.pen`, Storage · dark) with the design system's components.

use super::kit::{self, FILL, counted, line, mono, sans};
use crate::{
  app::{ctx::Ctx, theme::PaletteColor},
  components::{Column, Row, Spacer},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::Element,
  persistent_storage::PersistentStorageSnapshotEntry,
  ui::{Alert, AlertVariant, EmptyState, FontFamily, Length, ScrollArea, Tooltip, TooltipPart, icons},
};

/// The sizes the board gives the tab.
const HEADING_HEIGHT: f32 = 48.0;
const PADDING: f32 = kit::DETAIL_PADDING;
const ROW_HEIGHT: f32 = 36.0;
const KEY_WIDTH: f32 = 300.0;
const TYPE_WIDTH: f32 = 220.0;
const BYTES_WIDTH: f32 = 80.0;
const FIELD_HEIGHT: f32 = 20.0;
const FIELD_NAME_WIDTH: f32 = 184.0;
/// A chip of a type is as high as this, and what stands beside it on the row's first line is moved down to it.
const CHIP_PADDING: f32 = 2.0;

/// The tab: the storage of the inspected app as it is now, or why it could not be read.
pub(super) fn view(ctx: &mut Ctx) -> Element {
  let body = match ctx.app_ref().persistent_storage().snapshot() {
    Ok(entries) if entries.is_empty() => centred(
      EmptyState::new("Nothing is stored")
        .description("What the app keeps with `ctx.set_persistent_value` is listed here.")
        .icon(icons::DRIVE)
        .into(),
    ),
    Ok(entries) => list(ctx, &entries),
    Err(error) => centred(
      Alert::new("The storage could not be read")
        .description(&error.to_string())
        .variant(AlertVariant::Destructive)
        .into(),
    ),
  };
  Column::new()
    .size(FILL, FILL)
    .child(
      Row::new()
        .width(FILL)
        .height(HEADING_HEIGHT)
        .padding_horizontal(PADDING)
        .align_items(Alignment::Center)
        .border_bottom(line())
        .child(sans("Persistent Storage", 15.0, FontWeight::SemiBold)),
    )
    .child(Column::new().width(FILL).flex(1.0).child(body))
    .into()
}

fn centred(what: Element) -> Element {
  Column::new()
    .size(FILL, FILL)
    .padding(PADDING)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(what)
    .into()
}

/// How many bytes the values take together.
pub(crate) fn total_bytes(entries: &[PersistentStorageSnapshotEntry]) -> usize {
  entries.iter().map(|entry| entry.byte_len).sum()
}

/// The labels of the columns, the rows under them in what is left of the height, and the totals at the bottom.
fn list(ctx: &mut Ctx, entries: &[PersistentStorageSnapshotEntry]) -> Element {
  let label = |text: &str| sans(text, 12.0, FontWeight::Medium).color(PaletteColor::MutedForeground);
  let quiet = |text: &str| sans(text, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground);
  let mut rows = Column::new().width(FILL);
  for entry in entries {
    rows = rows.child(row(ctx, entry));
  }
  Column::new()
    .size(FILL, FILL)
    .child(
      Row::new()
        .width(FILL)
        .height(kit::LABELS_HEIGHT)
        .padding_horizontal(PADDING)
        .align_items(Alignment::Center)
        .border_bottom(line())
        .child(Row::new().width(KEY_WIDTH).child(label("Key")))
        // Over the text of a type, which stands in from its chip's edge.
        .child(Row::new().width(TYPE_WIDTH).padding_left(6.0).child(label("Type")))
        .child(Row::new().flex(1.0).child(label("Value")))
        .child(
          Row::new()
            .width(BYTES_WIDTH)
            .justify(Justify::End)
            .child(label("Bytes")),
        ),
    )
    .child(
      Column::new().width(FILL).flex(1.0).child(
        ScrollArea::new()
          .label("Stored values")
          .height(Length::Fill)
          .child(rows),
      ),
    )
    .child(
      Row::new()
        .width(FILL)
        .height(kit::DENSE_HEADER_HEIGHT)
        .padding_horizontal(PADDING)
        .align_items(Alignment::Center)
        .border_top(line())
        .child(quiet(&counted(entries.len(), "key")))
        .child(Spacer::new().flex(1.0))
        .child(quiet(&counted(total_bytes(entries), "byte"))),
    )
    .into()
}

/// A key's row: as high as a row of a list, or as its struct's fields need.
fn row(ctx: &mut Ctx, entry: &PersistentStorageSnapshotEntry) -> Row {
  // Everything stands on the row's first line, which a type's chip makes a little higher than its text.
  let first_line = |what: Row| what.padding_top(CHIP_PADDING);
  let value: Element = if entry.fields.is_empty() {
    mono(&entry.value, 13.0, FontWeight::Medium).into()
  } else {
    let mut fields = Column::new();
    for (name, value) in &entry.fields {
      fields = fields.child(
        Row::new()
          .height(FIELD_HEIGHT)
          .align_items(Alignment::Center)
          .child(
            Row::new()
              .width(FIELD_NAME_WIDTH)
              .child(mono(name, 13.0, FontWeight::Normal)),
          )
          .child(mono(value, 13.0, FontWeight::Medium)),
      );
    }
    fields.into()
  };
  Row::new()
    .id(format!("devtools-storage-{}", entry.key))
    .width(FILL)
    .min_height(ROW_HEIGHT)
    .padding_horizontal(PADDING)
    .padding_vertical((ROW_HEIGHT - 17.0 - 2.0 * CHIP_PADDING) / 2.0)
    .align_items(Alignment::Start)
    .border_bottom(line())
    .child(first_line(Row::new().width(KEY_WIDTH).child(mono(
      &entry.key,
      13.0,
      FontWeight::Normal,
    ))))
    .child(Row::new().width(TYPE_WIDTH).child(type_chip(ctx, entry)))
    .child(first_line(Row::new().flex(1.0).padding_right(32.0).child(value)))
    .child(first_line(Row::new().width(BYTES_WIDTH).justify(Justify::End).child(
      mono(&entry.byte_len.to_string(), 13.0, FontWeight::Normal).color(PaletteColor::MutedForeground),
    )))
}

/// The type of a value, in a chip. A type of the app's is called by its last name, and the pointer on it shows
/// where it is declared.
fn type_chip(ctx: &mut Ctx, entry: &PersistentStorageSnapshotEntry) -> Element {
  let chip = Row::new()
    .padding_horizontal(6.0)
    .padding_vertical(CHIP_PADDING)
    .rounded(crate::app::theme::RadiusSize::Sm)
    .foreground(PaletteColor::MutedForeground)
    .child(mono(&entry.type_name, 13.0, FontWeight::Normal));
  if entry.full_type_name == entry.type_name {
    return chip.into();
  }
  let chip = chip.hovered(|style| {
    style
      .background(PaletteColor::StateHover)
      .foreground(PaletteColor::Foreground)
  });
  Tooltip::new(&entry.full_type_name)
    .key(format!("devtools-storage-type-{}", entry.key))
    // A path is read whole on one line, in the type it is written in.
    .style(TooltipPart::Panel, |s| s.max_width(640.0))
    .style(TooltipPart::Label, |s| s.font_family(FontFamily::Mono).font_size(11.0))
    .mount(ctx, chip)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn entry(key: &str, byte_len: usize) -> PersistentStorageSnapshotEntry {
    PersistentStorageSnapshotEntry {
      key: key.to_owned(),
      type_name: "u32".to_owned(),
      full_type_name: "u32".to_owned(),
      value: "3".to_owned(),
      fields: Vec::new(),
      byte_len,
    }
  }

  #[test]
  fn the_totals_say_how_many_keys_and_bytes_there_are() {
    let entries = [entry("a", 5), entry("b", 224), entry("c", 45)];
    assert_eq!(total_bytes(&entries), 274);
    assert_eq!(counted(entries.len(), "key"), "3 keys");
    assert_eq!(counted(total_bytes(&entries), "byte"), "274 bytes");
    assert_eq!(counted(1, "key"), "1 key");
    assert_eq!(total_bytes(&[]), 0);
  }
}
