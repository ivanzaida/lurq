//! The page of the play button: its playground, the button bound to the page's signals, its sizes, its states,
//! and the overlay over a picture.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::RadiusSize},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::{Element, Gradient, GradientStop, color::Color},
  ui::{Interaction, PlayButton, PlayButtonSize, PlayButtonVariant},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .overflow_visible()
    .child(Row::new().overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .overflow_visible()
    .align_items(Alignment::End)
}

/// What stands in for a picture: the board's sunset, from the top down.
fn picture(width: f32, height: f32) -> Row {
  let stops = [
    ("#132544", 0.0),
    ("#1E3A8A", 0.3),
    ("#7C3AED", 0.58),
    ("#F472B6", 0.78),
    ("#F59E0B", 0.9),
    ("#3B1212", 1.0),
  ]
  .map(|(color, at)| GradientStop::at(Color::from_hex(color), at));
  Row::new()
    .size(width, height)
    .rounded(RadiusSize::Lg)
    .background_gradient(Gradient::linear(180.0, stops))
    .align_items(Alignment::Center)
    .justify(Justify::Center)
}

/// The sizes, as the playground offers them.
const SIZES: [(&str, PlayButtonSize); 4] = [
  ("Sm", PlayButtonSize::Sm),
  ("Md", PlayButtonSize::Md),
  ("Lg", PlayButtonSize::Lg),
  ("Xl", PlayButtonSize::Xl),
];

/// The variants, as the playground offers them.
const VARIANTS: [(&str, PlayButtonVariant); 2] = [
  ("Primary", PlayButtonVariant::Primary),
  ("Overlay", PlayButtonVariant::Overlay),
];

/// The side of the disc and of the glyph of each size.
const SIDES: [(f32, f32); 4] = [(32.0, 14.0), (40.0, 16.0), (48.0, 20.0), (64.0, 28.0)];

pub struct PlayButtonPage {
  frame: Frame,
  playground: Playground,
  size: Signal<usize>,
  variant: Signal<usize>,
  playing: Signal<bool>,
  replay: Signal<bool>,
  loading: Signal<bool>,
  disabled: Signal<bool>,
  play_label: Signal<String>,
  pause_label: Signal<String>,
  replay_label: Signal<String>,
  // The button the page's own signal drives.
  live: Signal<bool>,
  ended: Signal<bool>,
}

impl PlayButtonPage {
  /// The playground: the play button as the panel's props make it. A press sets the panel's `playing`, as a player
  /// would, and ends a replay.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let size = self.size.get().min(SIZES.len() - 1);
    let variant = self.variant.get().min(VARIANTS.len() - 1);
    let (playing, replay, loading, disabled) = (
      self.playing.get(),
      self.replay.get(),
      self.loading.get(),
      self.disabled.get(),
    );
    let (play_label, pause_label, replay_label) =
      (self.play_label.get(), self.pause_label.get(), self.replay_label.get());
    let plain = {
      let (play_label, pause_label, replay_label) = (play_label.clone(), pause_label.clone(), replay_label.clone());
      move || {
        PlayButton::new()
          .size(SIZES[size].1)
          .variant(VARIANTS[variant].1)
          .playing(playing)
          .replay(replay)
          .loading(loading)
          .disabled(disabled)
          .play_label(&play_label)
          .pause_label(&pause_label)
          .replay_label(&replay_label)
      }
    };
    let (record, set, ended) = (self.playground.recorder(), self.playing.clone(), self.replay.clone());
    let button = plain().force(self.playground.forced()).on_change(move |next| {
      set.set(next);
      ended.set(false);
      record(format!("on_change({next})"));
    });
    let overlay = variant == 1;
    let shown = if overlay {
      let playground = &self.playground;
      Shown::built(move |_| {
        picture(160.0, 120.0)
          .overflow_visible()
          .child(playground.measure(button))
          .into()
      })
      .measured()
    } else {
      Shown::of(button)
    };

    let mut code = vec!["PlayButton::new()".to_owned()];
    if size != 1 {
      code.push(format!("    .size(PlayButtonSize::{})", SIZES[size].0));
    }
    if overlay {
      code.push("    .variant(PlayButtonVariant::Overlay)".to_owned());
    }
    code.push("    .playing(playing.get())".to_owned());
    for (on, call) in [
      (replay, ".replay(true)"),
      (loading, ".loading(true)"),
      (disabled, ".disabled(true)"),
    ] {
      if on {
        code.push(format!("    {call}"));
      }
    }
    for (said, default, call) in [
      (&play_label, "Play", "play_label"),
      (&pause_label, "Pause", "pause_label"),
      (&replay_label, "Replay", "replay_label"),
    ] {
      if said != default {
        code.push(format!("    .{call}({said:?})"));
      }
    }
    code.push("    .on_change(move |next| playing.set(next))".to_owned());

    let (side, icon) = SIDES[size];
    let glyph = if loading {
      "spinner, turning"
    } else if replay {
      "rotate-left"
    } else if playing {
      "pause"
    } else {
      "play"
    };
    let tree = vec![
      format!("root   {}   round", self.playground.measured()),
      format!("└─ icon  {icon} × {icon} px   {glyph}"),
    ];
    let named = if replay {
      replay_label.clone()
    } else if playing {
      pause_label.clone()
    } else {
      play_label.clone()
    };

    let (pick_size, pick_variant) = (self.size.clone(), self.variant.clone());
    let labels = [
      (self.play_label.clone(), "Play"),
      (self.pause_label.clone(), "Pause"),
      (self.replay_label.clone(), "Replay"),
    ];
    let (to_size, to_variant) = (self.size.clone(), self.variant.clone());
    let flags = [
      self.playing.clone(),
      self.replay.clone(),
      self.loading.clone(),
      self.disabled.clone(),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Play Button",
        make: Arc::new(move || plain().into()),
        shown,
        fill: false,
        forceable: true,
        facts: format!("{side} round · called {named:?}"),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop(
            "size",
            "PlayButtonSize",
            self
              .playground
              .select(0, &SIZES.map(|(name, _)| (name, name)), &SIZES[size].0, move |name| {
                pick_size.set(SIZES.iter().position(|(held, _)| *held == name).unwrap_or(1))
              }),
          ),
          prop(
            "variant",
            "PlayButtonVariant",
            self.playground.select(
              1,
              &VARIANTS.map(|(name, _)| (name, name)),
              &VARIANTS[variant].0,
              move |name| pick_variant.set(VARIANTS.iter().position(|(held, _)| *held == name).unwrap_or(0)),
            ),
          ),
          prop("play_label", "&str", field(&self.play_label)),
          prop("pause_label", "&str", field(&self.pause_label)),
          prop("replay_label", "&str", field(&self.replay_label)),
          divider(),
          flag("playing", &self.playing),
          flag("replay", &self.replay),
          flag("loading", &self.loading),
          flag("disabled", &self.disabled),
        ],
        changed: [
          size != 1,
          variant != 0,
          playing,
          replay,
          loading,
          disabled,
          play_label != "Play",
          pause_label != "Pause",
          replay_label != "Replay",
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_size.set(1);
          to_variant.set(0);
          for flag in &flags {
            flag.set(false);
          }
          for (label, default) in &labels {
            label.set((*default).to_owned());
          }
        }),
      },
    )
  }
}

impl Component for PlayButtonPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      size: ctx.signal(1),
      variant: ctx.signal(0),
      playing: ctx.signal(false),
      replay: ctx.signal(false),
      loading: ctx.signal(false),
      disabled: ctx.signal(false),
      play_label: ctx.signal("Play".to_owned()),
      pause_label: ctx.signal("Pause".to_owned()),
      replay_label: ctx.signal("Replay".to_owned()),
      live: ctx.signal(false),
      ended: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (live, ended) = (self.live.get(), self.ended.get());
    let (set, unend, end) = (self.live.clone(), self.ended.clone(), self.ended.clone());
    let says = if ended {
      "ended: a press plays it again"
    } else if live {
      "playing"
    } else {
      "paused"
    };
    let bound = items()
      .child(item(
        PlayButton::new()
          .size(PlayButtonSize::Lg)
          .playing(live)
          .replay(ended)
          .on_change(move |next| {
            set.set(next);
            unend.set(false);
          }),
        says,
      ))
      .child(item(
        lurq::ui::Button::new("End it")
          .variant(lurq::ui::ButtonVariant::Outline)
          .size(lurq::ui::ButtonSize::Sm)
          .disabled(ended)
          .on_click(move |_| end.set(true)),
        "what a player does at the end",
      ));
    let sizes = SIZES
      .iter()
      .zip(SIDES)
      .fold(items(), |row, ((name, size), (side, icon))| {
        row.child(item(
          PlayButton::new().size(*size),
          &format!("{} · {side} with {icon}", name.to_lowercase()),
        ))
      });
    let states = items()
      .child(item(PlayButton::new(), "paused"))
      .child(item(PlayButton::new().playing(true), "playing"))
      .child(item(PlayButton::new().replay(true), "replay"))
      .child(item(PlayButton::new().force(Some(Interaction::Hovered)), "hover"))
      .child(item(
        PlayButton::new().force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(PlayButton::new().loading(true), "loading"))
      .child(item(PlayButton::new().disabled(true), "disabled"));
    let overlay = |size: PlayButtonSize, width: f32, height: f32| {
      picture(width, height)
        .overflow_visible()
        .child(PlayButton::new().variant(PlayButtonVariant::Overlay).size(size))
    };
    let over = items()
      .child(item(overlay(PlayButtonSize::Xl, 240.0, 136.0), "overlay · 64 with 28"))
      .child(item(
        overlay(PlayButtonSize::Lg, 160.0, 90.0),
        "overlay · 48 with 20, compact",
      ))
      .child(item(
        picture(160.0, 90.0).overflow_visible().child(
          PlayButton::new()
            .variant(PlayButtonVariant::Overlay)
            .size(PlayButtonSize::Lg)
            .force(Some(Interaction::FocusVisible)),
        ),
        "focus-visible: a white ring",
      ));
    let rows = [
      (
        "playing",
        "bool",
        "false",
        "Whether what it plays is playing: the glyph is the pause one. The button does not keep it: the app says so.",
      ),
      (
        "on_change",
        "impl Fn(bool)",
        "",
        "Called on a press with what it is to be: playing if it was paused or had ended. Not while it is loading \
         or disabled.",
      ),
      (
        "replay",
        "bool",
        "false",
        "What it played has ended: the glyph is rotate-left, and a press tells true.",
      ),
      (
        "loading",
        "bool",
        "false",
        "What it would play is still being fetched: the spinner turns, the disc is at 75%, and a press does nothing.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity; it takes no input and Tab passes it by.",
      ),
      (
        "size",
        "PlayButtonSize",
        "Md",
        "Sm 32 with an icon of 14, Md 40 with 16, Lg 48 with 20, Xl 64 with 28.",
      ),
      (
        "variant",
        "PlayButtonVariant",
        "Primary",
        "Primary on a surface of the theme; Overlay over a picture: black at 60% with a white glyph, the same in \
         both appearances.",
      ),
      (
        "play_label",
        "&str",
        "\"Play\"",
        "What it is called while a press would play.",
      ),
      (
        "pause_label",
        "&str",
        "\"Pause\"",
        "What it is called while a press would pause.",
      ),
      (
        "replay_label",
        "&str",
        "\"Replay\"",
        "What it is called while a press would play again from the start.",
      ),
      (
        "style / rules / state",
        "PlayButtonPart, Recipe, Name",
        "",
        "A part restyled, the rules of the component it is in, a state the theme's recipe names.",
      ),
      (
        "force",
        "Option<Interaction>",
        "None",
        "Draws it hovered, pressed or focused with nothing pointing at it.",
      ),
      (
        "id / key",
        "impl Into<Arc<str>>",
        "",
        "The id of the disc; what tells it apart from those beside it.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Play Button"],
        title: "Play Button",
        description: "The round control that starts and pauses what a player plays, and plays it again once it has \
                      ended.",
        import: Some("use lurq::ui::{PlayButton, PlayButtonSize, PlayButtonVariant};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of PlayButton is a control on the right. The button on the stage is live: a press sets the \
           panel's playing, as a player would, and the Events tab hears what it told. The overlay stands on a \
           gradient in place of a picture.",
          playground,
        ),
        Section::new(
          "Bound to a signal",
          "The page holds whether it plays. A press tells what it is to be and the page sets it; End it stands for \
           the end of what plays, after which the glyph is rotate-left and a press plays it again. Tab reaches it, \
           and Space or Enter presses it.",
          self.frame.demo(
            0,
            bound,
            "PlayButton::new().playing(playing.get())",
            &[
              "// playing, ended: Signal<bool>",
              "PlayButton::new()",
              "    .size(PlayButtonSize::Lg)",
              "    .playing(playing.get())",
              "    .replay(ended.get())",
              "    .on_change(move |next| {",
              "        playing.set(next);",
              "        ended.set(false);",
              "    })",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "Four sizes, the glyph growing with the disc. Md is the default; Xl is for a touch screen and the middle \
           of a picture.",
          self.frame.demo(
            1,
            sizes,
            ".size(PlayButtonSize::Lg)",
            &[
              "PlayButton::new().size(PlayButtonSize::Sm)",
              "PlayButton::new()",
              "PlayButton::new().size(PlayButtonSize::Lg)",
              "PlayButton::new().size(PlayButtonSize::Xl)",
            ],
          ),
        ),
        Section::new(
          "States",
          "Playing swaps the play glyph for pause, replay for rotate-left. Under the pointer it is the primary hover \
           color; the ring of the focus is drawn from the keyboard. Loading, the spinner turns at 75% and a press \
           does nothing; disabled it is at half.",
          self.frame.demo(
            2,
            states,
            ".playing(true)",
            &[
              "PlayButton::new()",
              "PlayButton::new().playing(true)",
              "PlayButton::new().replay(true)",
              "",
              "// Drawn hovered or focused with nothing pointing at it: for a gallery, a test.",
              "PlayButton::new().force(Some(Interaction::Hovered))",
              "PlayButton::new().force(Some(Interaction::FocusVisible))",
              "",
              "PlayButton::new().loading(true)",
              "PlayButton::new().disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Over a picture",
          "In the middle of a paused video: a disc of black at 60% with a white glyph and a white ring, the same in \
           a light and a dark theme, since the picture is not themed. 64 in a player, 48 in a compact one.",
          self.frame.demo(
            3,
            over,
            ".variant(PlayButtonVariant::Overlay)",
            &[
              "PlayButton::new()",
              "    .variant(PlayButtonVariant::Overlay)",
              "    .size(PlayButtonSize::Xl)",
              "    .playing(playing.get())",
              "    .on_change(move |next| playing.set(next))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a play button is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
