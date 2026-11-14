//! The page of the progress: its playground, a bar that moves to a new value,
//! its variants and sizes, a label with the value, the bar that does not know
//! how far the work is, the top bar, the ring, and a progress restyled into a
//! meter.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{
    Button, ButtonSize, ButtonVariant, Progress, ProgressPart, ProgressSize, ProgressVariant, RingProgress, Slider,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, quiet},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(10.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(32.0).wrap().align_items(Alignment::End)
}

const VARIANTS: [(&str, ProgressVariant); 4] = [
  ("Default", ProgressVariant::Default),
  ("Success", ProgressVariant::Success),
  ("Warning", ProgressVariant::Warning),
  ("Destructive", ProgressVariant::Destructive),
];

const SIZES: [(&str, ProgressSize); 3] = [
  ("Lg", ProgressSize::Lg),
  ("Md", ProgressSize::Md),
  ("Sm", ProgressSize::Sm),
];

/// The name of a variant or a size as the code says it.
fn named<T: PartialEq + Copy>(options: &[(&'static str, T)], chosen: T) -> &'static str {
  options
    .iter()
    .find(|(_, option)| *option == chosen)
    .map_or("", |(name, _)| name)
}

pub struct ProgressPage {
  frame: Frame,
  playground: Playground,
  value: Signal<i32>,
  indeterminate: Signal<bool>,
  variant: Signal<ProgressVariant>,
  size: Signal<ProgressSize>,
  label: Signal<String>,
  show_value: Signal<bool>,
  top_bar: Signal<bool>,
  ring: Signal<bool>,
  /// The bar of the first section, which a press moves on.
  upload: Signal<i32>,
}

impl ProgressPage {
  /// The playground: the progress as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (value, indeterminate) = (self.value.get(), self.indeterminate.get());
    let (variant, size, label) = (self.variant.get(), self.size.get(), self.label.get());
    let (show_value, top_bar, ring) = (self.show_value.get(), self.top_bar.get(), self.ring.get());
    let fraction = value as f32 / 100.0;
    let plain = {
      let label = label.clone();
      move || -> Element {
        if ring {
          let mut ring = RingProgress::new(fraction)
            .variant(variant)
            .size(size)
            .show_value(show_value);
          if !label.is_empty() {
            ring = ring.label(&label);
          }
          return ring.into();
        }
        let mut progress = if indeterminate {
          Progress::indeterminate()
        } else {
          Progress::new(fraction)
        }
        .variant(variant)
        .size(size)
        .show_value(show_value)
        .top_bar(top_bar);
        if !label.is_empty() {
          progress = progress.label(&label);
        }
        if top_bar {
          progress = progress.width(360.0);
        }
        progress.into()
      }
    };

    let mut code = vec![if ring {
      format!("RingProgress::new({fraction:?})")
    } else if indeterminate {
      "Progress::indeterminate()".to_owned()
    } else {
      format!("Progress::new({fraction:?})")
    }];
    if variant != ProgressVariant::Default {
      code.push(format!("    .variant(ProgressVariant::{})", named(&VARIANTS, variant)));
    }
    if size != ProgressSize::Lg {
      code.push(format!("    .size(ProgressSize::{})", named(&SIZES, size)));
    }
    if !label.is_empty() {
      code.push(format!("    .label({label:?})"));
    }
    if show_value != ring {
      code.push(format!("    .show_value({show_value})"));
    }
    if top_bar && !ring {
      code.push("    .top_bar(true)".to_owned());
      code.push("    .width(360.0)".to_owned());
    }

    let tree = if ring {
      vec![
        format!("root       {}", self.playground.measured()),
        "├─ track   a ring, a tenth of the side   secondary".to_owned(),
        format!(
          "├─ fill    one dash, {value}% from the top   {}",
          named(&VARIANTS, variant).to_lowercase()
        ),
        "└─ middle  nothing painted".to_owned(),
        format!(
          "   └─ value  {}",
          if show_value {
            format!("\"{value}%\"")
          } else {
            "hidden".to_owned()
          }
        ),
      ]
    } else {
      let mut tree = vec![format!("root       {}", self.playground.measured())];
      if !label.is_empty() || (show_value && !indeterminate) {
        tree.push("├─ header  label · value".to_owned());
      }
      tree.push("└─ track   secondary".to_owned());
      tree.push(if indeterminate {
        "   └─ fill  37.5% crossing the track".to_owned()
      } else {
        format!("   └─ fill  {value}% of the track")
      });
      tree
    };

    let to_value = self.value.clone();
    let set_value = self.value.clone();
    let value_control = Slider::new(set_value).label("value").length(160.0);
    let (to_variant, set_variant) = (self.variant.clone(), self.variant.clone());
    let (to_size, set_size) = (self.size.clone(), self.size.clone());
    let to_label = self.label.clone();
    let flags = [
      self.indeterminate.clone(),
      self.show_value.clone(),
      self.top_bar.clone(),
      self.ring.clone(),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Progress",
        make: Arc::new(plain.clone()),
        shown: plain(),
        fill: false,
        forceable: false,
        facts: if indeterminate && !ring {
          "not known".to_owned()
        } else {
          format!("value {value}%")
        },
        event: "",
        code,
        tree,
        controls: vec![
          prop("value", "impl Into<f64>", value_control),
          flag("indeterminate", &self.indeterminate),
          divider(),
          prop(
            "variant",
            "ProgressVariant",
            self
              .playground
              .select(0, &VARIANTS, &variant, move |next| set_variant.set(next)),
          ),
          prop(
            "size",
            "ProgressSize",
            self.playground.select(1, &SIZES, &size, move |next| set_size.set(next)),
          ),
          divider(),
          prop("label", "&str", field(&self.label)),
          flag("show_value", &self.show_value),
          flag("top_bar", &self.top_bar),
          divider(),
          flag("ring", &self.ring),
        ],
        changed: [
          value != 60,
          indeterminate,
          variant != ProgressVariant::Default,
          size != ProgressSize::Lg,
          label != "Uploading theme.json",
          show_value,
          top_bar,
          ring,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_value.set(60);
          to_variant.set(ProgressVariant::Default);
          to_size.set(ProgressSize::Lg);
          to_label.set("Uploading theme.json".to_owned());
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for ProgressPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      value: ctx.signal(60),
      indeterminate: ctx.signal(false),
      variant: ctx.signal(ProgressVariant::Default),
      size: ctx.signal(ProgressSize::Lg),
      label: ctx.signal("Uploading theme.json".to_owned()),
      show_value: ctx.signal(false),
      top_bar: ctx.signal(false),
      ring: ctx.signal(false),
      upload: ctx.signal(30),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let upload = self.upload.get();
    let step = self.upload.clone();
    let moving = items()
      .child(item(
        Progress::new(upload)
          .max(100)
          .label("Uploading theme.json")
          .show_value(true),
        "progress · moves to a new value",
      ))
      .child(item(RingProgress::new(upload).max(100), "ring · moves round"))
      .child(
        Button::new("Upload more")
          .variant(ButtonVariant::Outline)
          .size(ButtonSize::Sm)
          .on_click(move |_| step.update(|done| *done = if *done >= 100 { 0 } else { *done + 20 })),
      );
    let values = items()
      .child(item(Progress::new(0.6), "progress · 60%"))
      .child(item(Progress::new(0.25), "progress · 25%"));
    let variants = items()
      .child(item(
        Progress::new(0.83).variant(ProgressVariant::Success),
        "progress · success",
      ))
      .child(item(
        Progress::new(0.62).variant(ProgressVariant::Warning),
        "progress · warning",
      ))
      .child(item(
        Progress::new(0.25).variant(ProgressVariant::Destructive),
        "progress · destructive",
      ));
    let sizes = items()
      .child(item(Progress::new(0.5).size(ProgressSize::Sm), "sm · 4"))
      .child(item(Progress::new(0.5).size(ProgressSize::Md), "md · 6"))
      .child(item(Progress::new(0.5), "lg · 8"));
    let labeled = items()
      .child(item(
        Progress::new(0.62)
          .label("Uploading theme.json")
          .show_value(true)
          .size(ProgressSize::Md),
        "progress with label",
      ))
      .child(item(
        Progress::new(42)
          .max(128)
          .label("Copying")
          .show_value(true)
          .size(ProgressSize::Md)
          .format(|done, all| format!("{done} of {all}")),
        "the app's format",
      ));
    let unknown = items()
      .child(item(Progress::indeterminate().size(ProgressSize::Sm), "linear"))
      .child(item(
        Progress::indeterminate().label("Connecting").size(ProgressSize::Md),
        "with a label",
      ));
    let surface = Column::new()
      .width(360.0)
      .height(96.0)
      .clip()
      .rounded(RadiusSize::Lg)
      .background(PaletteColor::Background)
      .border(line())
      .child(Progress::new(0.6).top_bar(true))
      .child(
        Column::new()
          .width(FILL)
          .flex(1.0)
          .align_items(Alignment::Center)
          .justify(Justify::Center)
          .child(quiet("Page content", 11.0)),
      );
    let top_bar = items().child(item(surface, "top bar"));
    let rings = items()
      .child(item(RingProgress::new(0.7), "ring progress"))
      .child(item(RingProgress::new(0.45).size(ProgressSize::Md), "md · 56"))
      .child(item(
        RingProgress::new(0.9)
          .size(ProgressSize::Sm)
          .variant(ProgressVariant::Success),
        "sm · 40 · success",
      ))
      .child(item(
        RingProgress::new(96)
          .max(128)
          .variant(ProgressVariant::Warning)
          .format(|used, _| format!("{used}G")),
        "the app's format",
      ));
    let meter = items().child(item(
      Progress::new(96)
        .max(128)
        .label("Disk space")
        .show_value(true)
        .variant(ProgressVariant::Warning)
        .width(260.0)
        .format(|used, all| format!("{used} of {all} GB"))
        .style(ProgressPart::Track, |s| s.radius(2.0))
        .style(ProgressPart::Fill, |s| s.radius(2.0)),
      "a meter",
    ));

    let rows = [
      (
        "new",
        "impl Into<f64>",
        "",
        "How far the work is: a fraction of one, or of max. Kept between nothing and max.",
      ),
      (
        "indeterminate",
        "fn",
        "",
        "Work whose end is not known: a segment crosses the rail; it stands still while motion is reduced.",
      ),
      ("value", "impl Into<f64>", "", "Sets how far the work is."),
      ("max", "impl Into<f64>", "1", "The whole of the work."),
      (
        "variant",
        "ProgressVariant",
        "Default",
        "The color of what is filled: primary, success, warning or destructive.",
      ),
      (
        "size",
        "ProgressSize",
        "Lg",
        "The rail's height, 4, 6 or 8; a ring of 40, 56 or 72.",
      ),
      (
        "label",
        "&str",
        "",
        "What the work is, over the start of the rail, and what the progress is called.",
      ),
      (
        "show_value",
        "bool",
        "false",
        "The value over the end of the rail; a ring shows it in its middle unless told not to.",
      ),
      (
        "format",
        "impl Fn(f32, f32) -> String",
        "",
        "Says the value, given the value and max, in place of a percentage.",
      ),
      (
        "top_bar",
        "bool",
        "false",
        "The thin bar across the top of a window or a surface: 3 high, no rail.",
      ),
      (
        "width",
        "impl Into<Length>",
        "240",
        "How wide the progress is; all the room there is for a top bar.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Progress"],
        title: "Progress",
        description: "How far a piece of work is: a file uploading, items being copied. A bar along a rail, a thin \
                      bar across the top of a window, or a ring with the value in its middle.",
        import: Some("use lurq::ui::{Progress, RingProgress};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Progress is a control on the right; ring shows a RingProgress with the same value, \
           variant, size and label. Move the value: what is filled moves to it.",
          playground,
        ),
        Section::new(
          "Progress",
          "The page keeps the value, in a signal. What is filled moves to a new value over the theme's slow \
           duration, and the ring's arc moves round; at once while motion is reduced.",
          self.frame.demo(
            0,
            moving,
            "Progress::new(done).max(100)",
            &[
              "// done: Signal<i32>",
              "Progress::new(done.get())",
              "    .max(100)",
              "    .label(\"Uploading theme.json\")",
              "    .show_value(true)",
              "",
              "RingProgress::new(done.get()).max(100)",
            ],
          ),
        ),
        Section::new(
          "Values",
          "A rail of 240 by 8 in the secondary color, filled in the primary color from its start.",
          self.frame.demo(
            1,
            values,
            "Progress::new(0.6)",
            &["Progress::new(0.6)", "Progress::new(0.25)"],
          ),
        ),
        Section::new(
          "Variants",
          "What the work came to, or how much a share is: success, warning, destructive.",
          self.frame.demo(
            2,
            variants,
            ".variant(ProgressVariant::Success)",
            &[
              "Progress::new(0.83).variant(ProgressVariant::Success)",
              "Progress::new(0.62).variant(ProgressVariant::Warning)",
              "Progress::new(0.25).variant(ProgressVariant::Destructive)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "A rail of 4 in a row of a list, 6 under a label or in a dialog, 8 on its own.",
          self.frame.demo(
            3,
            sizes,
            ".size(ProgressSize::Md)",
            &[
              "Progress::new(0.5).size(ProgressSize::Sm)",
              "Progress::new(0.5).size(ProgressSize::Md)",
              "Progress::new(0.5)",
            ],
          ),
        ),
        Section::new(
          "With a label",
          "The label over the start of the rail, the value over its end: a percentage, or what the app says.",
          self.frame.demo(
            4,
            labeled,
            ".label(\"Uploading theme.json\").show_value(true)",
            &[
              "Progress::new(0.62)",
              "    .label(\"Uploading theme.json\")",
              "    .show_value(true)",
              "    .size(ProgressSize::Md)",
              "",
              "Progress::new(42)",
              "    .max(128)",
              "    .label(\"Copying\")",
              "    .show_value(true)",
              "    .size(ProgressSize::Md)",
              "    .format(|done, all| format!(\"{done} of {all}\"))",
            ],
          ),
        ),
        Section::new(
          "Not known",
          "Work whose end is not known: a segment crosses the rail again and again, and no value is said. While \
           motion is reduced the segment stands still, away from the start of the rail.",
          self.frame.demo(
            5,
            unknown,
            "Progress::indeterminate()",
            &[
              "Progress::indeterminate().size(ProgressSize::Sm)",
              "Progress::indeterminate().label(\"Connecting\").size(ProgressSize::Md)",
            ],
          ),
        ),
        Section::new(
          "Top bar",
          "The thin bar across the top of a window or a surface while a page loads: 3 high, with no rail, as wide \
           as what holds it. Put it first in what it is the top of.",
          self.frame.demo(
            6,
            top_bar,
            ".top_bar(true)",
            &[
              "Column::new()",
              "    .child(Progress::new(0.6).top_bar(true))",
              "    .child(page)",
            ],
          ),
        ),
        Section::new(
          "Ring",
          "The value around a circle, from the top and clockwise, with it in the middle: two round borders one \
           over the other, the arc's cut to one dash, and nothing painted in the middle, so a ring sits on any \
           surface.",
          self.frame.demo(
            7,
            rings,
            "RingProgress::new(0.7)",
            &[
              "RingProgress::new(0.7)",
              "RingProgress::new(0.45).size(ProgressSize::Md)",
              "RingProgress::new(0.9).size(ProgressSize::Sm).variant(ProgressVariant::Success)",
              "RingProgress::new(96)",
              "    .max(128)",
              "    .variant(ProgressVariant::Warning)",
              "    .format(|used, _| format!(\"{used}G\"))",
            ],
          ),
        ),
        Section::new(
          "As a meter",
          "A share of something, not work under way: the same parts, restyled where it is used.",
          self.frame.demo(
            8,
            meter,
            ".style(ProgressPart::Fill, ..)",
            &[
              "Progress::new(96)",
              "    .max(128)",
              "    .label(\"Disk space\")",
              "    .show_value(true)",
              "    .variant(ProgressVariant::Warning)",
              "    .width(260.0)",
              "    .format(|used, all| format!(\"{used} of {all} GB\"))",
              "    .style(ProgressPart::Track, |s| s.radius(2.0))",
              "    .style(ProgressPart::Fill, |s| s.radius(2.0))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a progress is given. A ring takes the same, apart from indeterminate, top_bar and width.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
