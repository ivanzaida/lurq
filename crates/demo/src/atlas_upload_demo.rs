use std::time::Duration;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::{Element, color::Color, dimension::Dimension},
};

use crate::style::{BG, BORDER, PRIMARY, SECONDARY, SURFACE, TEXT, TEXT_MUTED, WARNING, text};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CONTENT_PAD: f32 = 32.0;
const CARD_RADIUS: f32 = 8.0;

#[derive(Clone, lurq::DevtoolsInspectable)]
pub(crate) struct AtlasUploadProbeProps;

impl PartialEq for AtlasUploadProbeProps {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

pub(crate) struct AtlasUploadProbe {
  phase: Signal<usize>,
}

impl Component for AtlasUploadProbe {
  type Props = AtlasUploadProbeProps;

  fn create(ctx: &mut Ctx) -> Self {
    let phase = ctx.signal(0usize);

    for (delay_ms, next_phase) in [(850, 1usize), (1700, 2), (2550, 3)] {
      let phase_signal = phase.clone();
      ctx
        .create_timeout(Duration::from_millis(delay_ms), move || {
          phase_signal.set(next_phase);
        })
        .start();
    }

    Self { phase }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let phase = self.phase.get();
    let text_blocks = phase_text_blocks(phase);

    Column::new()
      .spacing(24.0)
      .child(text("Atlas Upload Probe", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
      .child(status_row(phase))
      .child(active_sample(phase))
      .child(
        Column::new()
          .spacing(14.0)
          .with_children(text_blocks.into_iter().map(text_sample))
          .padding(24.0)
          .width(FILL_WIDTH)
          .background(SURFACE)
          .border_inside(1.0, Color::from_hex(BORDER))
          .rounded(CARD_RADIUS),
      )
      .padding(CONTENT_PAD)
      .width(FILL_WIDTH)
      .background(BG)
  }
}

fn status_row(phase: usize) -> Element {
  Row::new()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(status_pill("warm", true, PRIMARY))
    .child(status_pill("latin", phase >= 1, SECONDARY))
    .child(status_pill("symbols", phase >= 2, WARNING))
    .child(status_pill("mixed", phase >= 3, "#22c55e"))
    .width(FILL_WIDTH)
    .into()
}

fn status_pill(label: &str, active: bool, fill: &'static str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .child(text(
      label,
      12.0,
      FontWeight::Bold,
      if active { "#ffffff" } else { TEXT_MUTED },
    ))
    .height(28.0)
    .padding_horizontal(12.0)
    .background(if active { fill } else { "#111827" })
    .border_inside(1.0, Color::from_hex(if active { fill } else { BORDER }))
    .rounded(6.0)
    .into()
}

fn text_sample(content: &'static str) -> Element {
  text(content, 18.0, FontWeight::Normal, TEXT).width(FILL_WIDTH).into()
}

fn active_sample(phase: usize) -> Element {
  Column::new()
    .spacing(8.0)
    .child(text("Active glyph update", 13.0, FontWeight::Bold, TEXT_MUTED).width(FILL_WIDTH))
    .child(text(active_sample_text(phase), 24.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
    .padding(18.0)
    .width(FILL_WIDTH)
    .background("#111827")
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn active_sample_text(phase: usize) -> &'static str {
  match phase {
    0 => "ASCII warmup: quick cached glyphs 1234567890",
    1 => "Latin + math: café façade Ω Δ λ ± × ÷ √ ∫ ∑",
    2 => "Cyrillic + arrows: Привет мир ← ↑ → ↓ ↔ ⇧",
    _ => "Dense mix: Žižek Γραφή Журнал € £ ¥ ₹ ₿ ½ ¼ ¾",
  }
}

fn phase_text_blocks(phase: usize) -> Vec<&'static str> {
  let mut blocks = vec![
    "Warm atlas baseline: The quick brown fox jumps over 1234567890 cached letters.",
    "Stable paragraph: repeated ASCII text keeps the existing glyph atlas hot across frames.",
    "Column metrics: width, wrap, baseline, ascent, descent, kerning, tracking.",
  ];

  if phase >= 1 {
    blocks.push("Latin extensions: café naïve façade coöperate São Tomé smörgåsbord Łódź.");
    blocks.push("Math marks: ± × ÷ ≈ ≠ ≤ ≥ √ ∫ ∑ ∞ μ π Ω Δ λ.");
  }

  if phase >= 2 {
    blocks.push("Cyrillic and Greek: Привет мир, быстрый текст, Καλημέρα κόσμε.");
    blocks.push("Currency and arrows: € £ ¥ ₹ ₿ ← ↑ → ↓ ↔ ⇧ ⌘ ⌥ ⌫.");
  }

  if phase >= 3 {
    blocks.push("Dense mix: fjord vext quiz glyphs with Žižek, Γραφή, Журнал, ½ ¼ ¾.");
    blocks.push("Final line: atlas dirty uploads should stay smaller than a full texture after warmup.");
  }

  blocks
}
