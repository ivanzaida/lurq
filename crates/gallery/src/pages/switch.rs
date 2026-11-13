//! The page of the switch: its playground, the switch bound to a signal, its
//! states, and the track alone in a row of settings.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::Element,
  ui::{Interaction, Switch},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, quiet, text},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

pub struct SwitchPage {
  frame: Frame,
  playground: Playground,
  label: Signal<String>,
  on: Signal<bool>,
  label_hidden: Signal<bool>,
  disabled: Signal<bool>,
  dark: Signal<bool>,
  motion: Signal<bool>,
  sync: Signal<bool>,
  paused: Signal<bool>,
}

impl SwitchPage {
  /// The playground: the switch as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (label, on) = (self.label.get(), self.on.get());
    let (label_hidden, disabled) = (self.label_hidden.get(), self.disabled.get());
    let (record, set) = (self.playground.recorder(), self.on.clone());
    let plain = {
      let label = label.clone();
      move || Switch::new(&label).on(on).label_hidden(label_hidden).disabled(disabled)
    };
    let switch = plain().force(self.playground.forced()).on_change(move |next| {
      set.set(next);
      record(format!("on_change({next})"));
    });

    let mut code = vec![format!("Switch::new({label:?})")];
    if label_hidden {
      code.push("    .label_hidden(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .on(dark.get())".to_owned());
    code.push("    .on_change(move |on| dark.set(on))".to_owned());

    let mut tree = vec![
      format!("root      {}", self.playground.measured()),
      format!(
        "{} track     36 × 20 px   {}",
        if label_hidden { "└─" } else { "├─" },
        if on { "on" } else { "off" }
      ),
      format!(
        "{}  └─ thumb  16 × 16 px   at the {}",
        if label_hidden { " " } else { "│" },
        if on { "end" } else { "start" }
      ),
    ];
    if !label_hidden {
      tree.push(format!("└─ label     {label:?}"));
    }

    let to_label = self.label.clone();
    let flags = [self.on.clone(), self.label_hidden.clone(), self.disabled.clone()];
    self.playground.card(
      ctx,
      Spec {
        title: "Switch",
        make: Arc::new(move || plain().into()),
        shown: switch.into(),
        fill: false,
        forceable: true,
        facts: if on { "on".to_owned() } else { "off".to_owned() },
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop("label", "&str", field(&self.label)),
          divider(),
          flag("on", &self.on),
          flag("label_hidden", &self.label_hidden),
          flag("disabled", &self.disabled),
        ],
        changed: [on, label_hidden, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_label.set("Dark mode".to_owned());
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }

  /// A row of settings: what the setting is and what it does on the left, its switch at the end.
  fn setting(title: &str, about: &str, on: &Signal<bool>) -> Row {
    let set = on.clone();
    Row::new()
      .width(FILL)
      .padding_horizontal(16.0)
      .padding_vertical(12.0)
      .spacing(16.0)
      .align_items(Alignment::Center)
      .justify(Justify::SpaceBetween)
      .child(
        Column::new()
          .flex(1.0)
          .spacing(2.0)
          .child(text(title, 14.0, FontWeight::Medium))
          .child(text(about, 13.0, FontWeight::Normal).color(PaletteColor::MutedForeground)),
      )
      .child(
        Switch::new(title)
          .label_hidden(true)
          .on(on.get())
          .on_change(move |next| set.set(next)),
      )
  }
}

impl Component for SwitchPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      label: ctx.signal("Dark mode".to_owned()),
      on: ctx.signal(false),
      label_hidden: ctx.signal(false),
      disabled: ctx.signal(false),
      dark: ctx.signal(true),
      motion: ctx.signal(false),
      sync: ctx.signal(true),
      paused: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let bound = |label: &str, on: &Signal<bool>| {
      let set = on.clone();
      Switch::new(label).on(on.get()).on_change(move |next| set.set(next))
    };
    let says = |on: &Signal<bool>| {
      if on.get() {
        "the signal has: true"
      } else {
        "the signal has: false"
      }
    };
    let live = items()
      .child(item(bound("Dark mode", &self.dark), says(&self.dark)))
      .child(item(bound("Reduce motion", &self.motion), says(&self.motion)));
    let states = items()
      .child(item(Switch::new("Off"), "off"))
      .child(item(Switch::new("On").on(true), "on"))
      .child(item(
        Switch::new("Focused").force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(Switch::new("Disabled").disabled(true), "disabled"))
      .child(item(
        Switch::new("Disabled, on").on(true).disabled(true),
        "disabled, on",
      ));
    let settings = Column::new()
      .width(FILL)
      .clip()
      .rounded(RadiusSize::Md)
      .background(PaletteColor::Background)
      .border(line())
      .child(Self::setting(
        "Sync in background",
        "Keep this device up to date while the app is closed.",
        &self.sync,
      ))
      .child(
        Self::setting(
          "Pause notifications",
          "Nothing is shown until this is turned off again.",
          &self.paused,
        )
        .border_top(line()),
      );
    let rows = [
      (
        "label",
        "&str",
        "",
        "The text beside the track, and what the switch is called. A press of it is a press of the switch.",
      ),
      (
        "on",
        "bool",
        "false",
        "Whether it is on. The switch does not keep it: the app says so.",
      ),
      (
        "on_change",
        "impl Fn(bool)",
        "",
        "Called on a press with what the switch is to be: on if it was off.",
      ),
      (
        "label_hidden",
        "bool",
        "false",
        "The track alone, where the text is somewhere else. The label is still what the switch is called.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity, with its label; it takes no input.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Switch"],
        title: "Switch",
        description: "A setting that is on or off and takes effect at once: dark mode, sync in the background, \
                      notifications paused.",
        import: Some("use lurq::ui::Switch;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Switch is a control on the right. The switch on the stage is live: a press of its track or \
           of its label turns it, and the thumb slides to the other end.",
          playground,
        ),
        Section::new(
          "Switch",
          "The page holds whether each is on, in a signal. Tab reaches the track, and Space or Enter presses it.",
          self.frame.demo(
            0,
            live,
            "Switch::new(\"Dark mode\").on(dark.get()).on_change(..)",
            &[
              "// dark, motion: Signal<bool>",
              "let set = dark.clone();",
              "Switch::new(\"Dark mode\")",
              "    .on(dark.get())",
              "    .on_change(move |on| set.set(on))",
              "",
              "let set = motion.clone();",
              "Switch::new(\"Reduce motion\")",
              "    .on(motion.get())",
              "    .on_change(move |on| set.set(on))",
            ],
          ),
        ),
        Section::new(
          "States",
          "Off the track is the color of an input's edge, on it is the primary color. The ring of the focus is the \
           track's, and a disabled switch is dimmed with its label.",
          self.frame.demo(
            1,
            states,
            ".on(true)",
            &[
              "Switch::new(\"Off\")",
              "Switch::new(\"On\").on(true)",
              "",
              "// Drawn focused from the keyboard with nothing focused: for a gallery, a test.",
              "Switch::new(\"Focused\").force(Some(Interaction::FocusVisible))",
              "",
              "Switch::new(\"Disabled\").disabled(true)",
              "Switch::new(\"Disabled, on\").on(true).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "In a row of settings",
          "The track alone at the end of a row that says what the setting is. The switch is still called by its \
           label, which tooling that reads the tree is told.",
          self.frame.demo(
            2,
            settings,
            ".label_hidden(true)",
            &[
              "// sync: Signal<bool>",
              "let set = sync.clone();",
              "Row::new()",
              "    .width(Dimension::Pct(100.0))",
              "    .align_items(Alignment::Center)",
              "    .justify(Justify::SpaceBetween)",
              "    .child(",
              "        Column::new()",
              "            .child(Text::new(\"Sync in background\"))",
              "            .child(Text::new(\"Keep this device up to date while the app is closed.\")),",
              "    )",
              "    .child(",
              "        Switch::new(\"Sync in background\")",
              "            .label_hidden(true)",
              "            .on(sync.get())",
              "            .on_change(move |on| set.set(on)),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a switch is given.",
          props_table(&rows, &["label"], self.frame.room()),
        ),
      ],
    )
  }
}
