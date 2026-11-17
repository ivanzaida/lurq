//! The page of the glyph atlas: the texture the renderers keep every glyph they have drawn in, at each size and
//! weight, and send to the card only the part of it that new glyphs were put in. Its first section is the demo's
//! atlas upload probe, whole: it warms the atlas with ASCII, then brings in Latin extensions and signs of
//! mathematics, then Cyrillic, Greek, currencies and arrows, then a dense mix, on a timer from when the page is
//! made, as the demo's did, or at a press. It is what a profile of the atlas's uploads is taken of:
//! `cargo run -p gallery --features perf_profile -- --route /engine/glyph-atlas --profile-log`.
//!
//! The probe comes before the playground, which is the order of every other page turned: the probe's text has to
//! be in the window from its first frame, or its glyphs are not drawn when its steps come, and nothing is uploaded
//! for them.

use std::{collections::HashSet, sync::Arc, time::Duration};

use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, Timeout},
    theme::{PaletteColor, RadiusSize, TypographyStyle},
  },
  components::{Column, Row, Text},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{Badge, BadgeVariant, Button, ButtonSize, ButtonVariant},
};

use super::kit::{controls, named, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, mono, plain_block, props_table, text},
  playground::{Playground, Shown, Spec, prop},
};

/// When each step of the probe comes, from when the page is made, and the step it brings: the demo's times.
const STEPS: [(u64, usize); 3] = [(850, 1), (1700, 2), (2550, 3)];
/// The last step.
const LAST: usize = 3;

/// The line the probe draws large at `phase`: the glyphs that step brings, at 24 and bold.
fn active_sample_text(phase: usize) -> &'static str {
  match phase {
    0 => "ASCII warmup: quick cached glyphs 1234567890",
    1 => "Latin + math: café façade Ω Δ λ ± × ÷ √ ∫ ∑",
    2 => "Cyrillic + arrows: Привет мир ← ↑ → ↓ ↔ ⇧",
    _ => "Dense mix: Žižek Γραφή Журнал € £ ¥ ₹ ₿ ½ ¼ ¾",
  }
}

/// The paragraphs the probe draws at `phase`, at 18: three of ASCII from the start, and two more at each step.
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

/// The probe's four steps as marks, each lit once its step has come: in the demo's order and with its words.
fn status_row(phase: usize) -> Row {
  let pill = |label: &str, active: bool, variant: BadgeVariant| {
    Badge::new(label).variant(if active { variant } else { BadgeVariant::Outline })
  };
  Row::new()
    .width(FILL)
    .spacing(12.0)
    .wrap()
    .align_items(Alignment::Center)
    .child(pill("warm", true, BadgeVariant::Default))
    .child(pill("latin", phase >= 1, BadgeVariant::Info))
    .child(pill("symbols", phase >= 2, BadgeVariant::Warning))
    .child(pill("mixed", phase >= 3, BadgeVariant::Success))
}

/// The line of the step that came last, large, under what it is.
fn active_sample(phase: usize) -> Column {
  Column::new()
    .width(FILL)
    .spacing(8.0)
    .padding(18.0)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Muted)
    .border(line())
    .child(text("Active glyph update", 13.0, FontWeight::Bold).color(PaletteColor::MutedForeground))
    .child(text(active_sample_text(phase), 24.0, FontWeight::Bold).width(FILL))
}

/// The paragraphs of `phase`, one under another.
fn text_samples(phase: usize) -> Column {
  Column::new().width(FILL).spacing(14.0).with_children(
    phase_text_blocks(phase)
      .into_iter()
      .map(|content| text(content, 18.0, FontWeight::Normal).width(FILL)),
  )
}

/// The scripts of the playground: a line of each.
const SCRIPTS: [(&str, &str); 5] = [
  ("ASCII", "The quick brown fox jumps over the lazy dog 0123456789"),
  ("Latin", "Ærøskøbing, Łódź, Žižkov, São Tomé, Ðakovo, Şanlıurfa"),
  ("Cyrillic", "Співає соловейко, Щедрик щебетав, Ґанок і ящірка"),
  ("Greek", "Ξεσκεπάζω την ψυχοφθόρα βδελυγμία"),
  ("Symbols", "∀ε>0 ∃δ: ∑ ∏ ∮ ∇ ⊗ ⊕ ≡ ≅ ⇔ ↯ ★ ☂ ♞ ⚑ ✂"),
];
const SCRIPT_CHOICES: [(&str, usize); 5] = [
  ("ASCII", 0),
  ("Latin", 1),
  ("Cyrillic", 2),
  ("Greek", 3),
  ("Symbols", 4),
];
const SIZES: [(&str, i32); 4] = [("14", 14), ("18", 18), ("24", 24), ("40", 40)];
const WEIGHTS: [(&str, usize); 3] = [("Normal", 0), ("SemiBold", 1), ("Bold", 2)];
const WEIGHT_VALUES: [FontWeight; 3] = [FontWeight::Normal, FontWeight::SemiBold, FontWeight::Bold];

pub struct GlyphAtlasPage {
  frame: Frame,
  playground: Playground,
  /// The step of the probe: 0 warm, 1 Latin and mathematics, 2 Cyrillic, Greek, currencies and arrows, 3 mixed.
  phase: Signal<usize>,
  /// What brings each step on its time: started when the page is made, and again by Run again.
  timers: Vec<Timeout>,
  /// An index into [`SCRIPTS`].
  script: Signal<usize>,
  size: Signal<i32>,
  /// An index into [`WEIGHTS`].
  weight: Signal<usize>,
}

impl GlyphAtlasPage {
  /// The probe: its marks, the line of the last step and its paragraphs, with a button that brings the next step
  /// at once and one that runs it from the start.
  fn probe(&self) -> Column {
    let phase = self.phase.get().min(LAST);
    let next = {
      let (phase, timers) = (self.phase.clone(), self.timers.clone());
      move |_| {
        for timer in &timers {
          timer.cancel();
        }
        phase.update(|phase| *phase = (*phase + 1).min(LAST));
      }
    };
    let again = {
      let (phase, timers) = (self.phase.clone(), self.timers.clone());
      move |_| {
        phase.set(0);
        for timer in &timers {
          timer.restart();
        }
      }
    };
    let running = self.timers.iter().any(Timeout::is_active);
    Column::new()
      .width(FILL)
      .spacing(24.0)
      .child(
        controls()
          .align_items(Alignment::Center)
          .child(
            Button::new("Next step")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .disabled(phase >= LAST)
              .on_click(next),
          )
          .child(
            Button::new("Run again")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .icon("refresh")
              .on_click(again),
          )
          .child(
            mono(
              &format!("step {phase} of {LAST}{}", if running { ", on a timer" } else { "" }),
              12.0,
            )
            .color(PaletteColor::MutedForeground),
          ),
      )
      .child(status_row(phase))
      .child(active_sample(phase))
      .child(well().padding(24.0).child(text_samples(phase)))
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let (script, size, weight) = (
      self.script.get().min(SCRIPTS.len() - 1),
      self.size.get(),
      self.weight.get(),
    );
    let (name, sample) = SCRIPTS[script];
    let weight_name = WEIGHTS[weight.min(WEIGHTS.len() - 1)].0;
    let font_weight = WEIGHT_VALUES[weight.min(WEIGHT_VALUES.len() - 1)];
    let build = move || -> Element {
      let theme = lurq::ui::current_theme();
      let mut style = theme.typography_style(TypographyStyle::Body);
      style.font_size = size as f32;
      style.weight = font_weight;
      Column::new()
        .width(FILL)
        .child(Text::styled(sample, style).width(FILL))
        .into()
    };
    let glyphs = sample
      .chars()
      .filter(|glyph| !glyph.is_whitespace())
      .collect::<HashSet<_>>()
      .len();
    let code = vec![
      "let mut style = theme.typography_style(TypographyStyle::Body);".to_owned(),
      format!("style.font_size = {size}.0;"),
      format!("style.weight = FontWeight::{weight_name};"),
      format!("Text::styled(\"{sample}\", style)"),
    ];
    let (set_script, set_size, set_weight) = (self.script.clone(), self.size.clone(), self.weight.clone());
    let reset = {
      let (script, size, weight) = (self.script.clone(), self.size.clone(), self.weight.clone());
      Arc::new(move || {
        script.set(0);
        size.set(18);
        weight.set(0);
      })
    };
    let controls = vec![
      prop(
        "text",
        "&str",
        self
          .playground
          .select(0, &SCRIPT_CHOICES, &script, move |value| set_script.set(value)),
      ),
      prop(
        "font_size",
        "f32",
        self
          .playground
          .select(1, &SIZES, &size, move |value| set_size.set(value)),
      ),
      prop(
        "weight",
        "FontWeight",
        self
          .playground
          .select(2, &WEIGHTS, &weight, move |value| set_weight.set(value)),
      ),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Glyphs",
        make: Arc::new(build),
        shown: Shown::built(move |_| build()),
        fill: true,
        forceable: false,
        facts: format!("{glyphs} glyphs of {name}"),
        event: "",
        code,
        tree: vec![format!("text   {}", self.playground.measured())],
        controls,
        changed: [script != 0, size != 18, weight != 0]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

/// How to profile the probe, and what its log says.
fn profiling() -> Column {
  let commands = [
    "cargo run -p gallery --features perf_profile -- --route /engine/glyph-atlas --profile-log",
    "cargo run -p gallery --features perf_profile -- --route /engine/glyph-atlas --renderer dx12 --profile-log",
  ]
  .map(str::to_owned);
  let read = |name: &str, what: &str| named(name, text(what, 13.0, FontWeight::Normal).width(FILL));
  Column::new()
    .width(FILL)
    .spacing(16.0)
    .child(
      Column::new()
        .width(FILL)
        .rounded(RadiusSize::Md)
        .background(PaletteColor::Muted)
        .border(line())
        .child(plain_block(&commands)),
    )
    .child(read(
      "log",
      "target/perf_profile.log, or the file --profile-log=<file> names: a line for each frame presented.",
    ))
    .child(read(
      "atlas=",
      "atlas=<bytes>B <rects> rects <full> full: what the frame sent of the atlas to the card, in how many \
       rectangles, and how many times the whole of it.",
    ))
    .child(read(
      "first frame",
      "The atlas is made and sent whole: one full upload.",
    ))
    .child(read("warm frames", "Nothing new is drawn: no bytes, no rectangles."))
    .child(read(
      "each step",
      "Its new glyphs are packed into the atlas and only their rectangles are sent: no full upload.",
    ))
}

impl Component for GlyphAtlasPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let phase = ctx.signal(0usize);
    let timers = STEPS
      .into_iter()
      .map(|(delay_ms, next_phase)| {
        let phase_signal = phase.clone();
        let timer = ctx.create_timeout(Duration::from_millis(delay_ms), move || {
          phase_signal.set(next_phase);
        });
        timer.start();
        timer
      })
      .collect();
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      phase,
      timers,
      script: ctx.signal(0),
      size: ctx.signal(18),
      weight: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "font_size",
        "f32",
        "the theme's",
        "A glyph is kept in the atlas at each size it is drawn at, times the display's scale.",
      ),
      (
        "weight",
        "FontWeight",
        "Normal",
        "Each weight of a glyph is another entry, from another face where the family has one.",
      ),
      (
        "font_family",
        "Arc<str>",
        "the theme's",
        "A glyph of another family is another entry; a glyph the family has not is drawn from a fallback's face.",
      ),
    ];
    let probe = self.probe();
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Glyph Atlas"],
        title: "Glyph Atlas",
        description: "The texture every glyph drawn is kept in, at each size and weight. A glyph drawn for the first \
                      time is packed into it and only its rectangle is sent to the card. This page is the probe a \
                      profile of those uploads is taken of.",
        import: Some("use lurq::components::Text;"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "The probe",
          "Three paragraphs of ASCII warm the atlas. On a timer from when the page is made, at 0.85, 1.7 and 2.55 \
           seconds, it brings in Latin extensions and signs of mathematics, then Cyrillic, Greek, currencies and \
           arrows, then a dense mix: each step draws glyphs the atlas has not seen. Next step brings the next at \
           once. A run again uploads nothing: the glyphs are in the atlas, until the gallery starts again.",
          self.frame.demo(
            0,
            probe,
            "ctx.create_timeout(Duration::from_millis(850), ..)",
            &[
              "let phase = ctx.signal(0usize);",
              "for (delay_ms, next_phase) in [(850, 1usize), (1700, 2), (2550, 3)] {",
              "    let phase = phase.clone();",
              "    ctx.create_timeout(Duration::from_millis(delay_ms), move || phase.set(next_phase))",
              "        .start();",
              "}",
              "",
              "// at each render: the paragraphs of the steps that have come",
              "Column::new()",
              "    .spacing(14.0)",
              "    .with_children(phase_text_blocks(phase.get()).into_iter().map(|block| text(block, 18.0)))",
            ],
          ),
        ),
        Section::new(
          "Playground",
          "A line of a script at a size and a weight. Each that has not been drawn yet puts new glyphs into the \
           atlas: the same letters at another size are other glyphs.",
          playground,
        ),
        Section::new(
          "Profiling it",
          "Built with perf_profile, the gallery writes what each frame cost to a log; opened on this page, the log \
           is the probe's. A step that uploads the whole atlas again, or a warm frame that uploads anything, is \
           what the probe is there to catch.",
          self.frame.demo(
            1,
            profiling(),
            "--features perf_profile -- --route /engine/glyph-atlas --profile-log",
            &[
              "cargo run -p gallery --features perf_profile -- --route /engine/glyph-atlas --profile-log",
              "cargo run -p gallery --features perf_profile -- --route /engine/glyph-atlas --renderer dx12 \
               --profile-log",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What makes a glyph an entry of its own in the atlas.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
