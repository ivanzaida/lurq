//! The page of the video player: its playground, driven by a clock of the page's own, its states, its three forms,
//! the player without controls, and one with an app's own controls laid over the picture.

use std::{sync::Arc, time::Duration};

use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, Interval},
  },
  components::{Column, Row, Stack},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, Gradient, HitTestBehavior, color::Color, dimension::Dimension, gradient::GradientStop},
  ui::{
    Button, ButtonPart, ButtonShape, ButtonVariant, PlayButton, PlayButtonVariant, VideoPlayer, VideoPlayerSize, icons,
    media_time_text,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// How long the playground's video is, in seconds.
const LENGTH: f64 = 250.0;

/// How often the page's clock moves the position on while it plays.
const TICK: Duration = Duration::from_millis(250);

/// The forms the playground offers: the one its width gives it, or one it is told.
const SIZES: [(&str, Option<VideoPlayerSize>); 4] = [
  ("from the width", None),
  ("Lg", Some(VideoPlayerSize::Lg)),
  ("Md", Some(VideoPlayerSize::Md)),
  ("Sm", Some(VideoPlayerSize::Sm)),
];

/// The widths the playground offers, each with a height of 16 by 9.
const WIDTHS: [(&str, f32); 4] = [("640", 640.0), ("432", 432.0), ("320", 320.0), ("200", 200.0)];

fn hex(rgb: u32) -> Color {
  Color::new((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255)
}

/// A picture that stands for a frame of the video: the board's sunset, from the top down.
fn picture() -> Column {
  let stops = [
    (0x132544, 0.0),
    (0x1E3A8A, 0.3),
    (0x7C3AED, 0.58),
    (0xF472B6, 0.78),
    (0xF59E0B, 0.9),
    (0x3B1212, 1.0),
  ]
  .map(|(rgb, at)| GradientStop::at(hex(rgb), at));
  Column::new()
    .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
    .background_gradient(Gradient::linear(180.0, stops))
}

/// The poster's picture: the board's teal, from the top left.
fn poster() -> Column {
  let stops = [(0x0F2E1B, 0.0), (0x0D9488, 0.45), (0x132544, 1.0)].map(|(rgb, at)| GradientStop::at(hex(rgb), at));
  Column::new()
    .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
    .background_gradient(Gradient::linear(135.0, stops))
}

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
    .align_items(Alignment::End)
    .overflow_visible()
}

/// A player of the states' size, paused at 1:24 of 4:10, for a picture of a state.
fn state() -> VideoPlayer {
  VideoPlayer::new()
    .media(picture())
    .width(432.0)
    .height(243.0)
    .duration(LENGTH)
    .buffered(140.0)
    .position_controlled(84.0, |_| {})
}

pub struct VideoPlayerPage {
  frame: Frame,
  playground: Playground,
  size: Signal<usize>,
  width: Signal<usize>,
  playing: Signal<bool>,
  position: Signal<f32>,
  volume: Signal<f32>,
  muted: Signal<bool>,
  rate: Signal<f32>,
  captions: Signal<bool>,
  quality: Signal<Option<usize>>,
  fullscreen: Signal<bool>,
  picture_in_picture: Signal<bool>,
  waiting: Signal<bool>,
  ended: Signal<bool>,
  live: Signal<bool>,
  error: Signal<bool>,
  poster: Signal<bool>,
  controls: Signal<bool>,
  overlay: Signal<bool>,
  feed_playing: Signal<bool>,
  feed_muted: Signal<bool>,
  _clock: Interval,
}

impl VideoPlayerPage {
  /// The line of the captions for a time: the cue the app would have.
  fn cue(position: f64) -> &'static str {
    const CUES: [&str; 4] = [
      "The bar fades out while the video plays.",
      "Move the pointer and it comes back.",
      "Space or K plays and pauses; the arrows seek.",
      "C turns these captions off.",
    ];
    CUES[(position / 6.0) as usize % CUES.len()]
  }

  /// The playground: the player as the panel's props make it, its values held by the page and moved on by its clock.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let size = SIZES[self.size.get().min(SIZES.len() - 1)].1;
    let width = WIDTHS[self.width.get().min(WIDTHS.len() - 1)].1;
    let (waiting, ended, live, error) = (self.waiting.get(), self.ended.get(), self.live.get(), self.error.get());
    let (poster_on, controls, overlay) = (self.poster.get(), self.controls.get(), self.overlay.get());
    let position = f64::from(self.position.get());
    let caption = Self::cue(position);
    let make = {
      let (playing, position, volume, muted, rate) = (
        self.playing.clone(),
        self.position.clone(),
        self.volume.clone(),
        self.muted.clone(),
        self.rate.clone(),
      );
      let (captions, quality, fullscreen, pip) = (
        self.captions.clone(),
        self.quality.clone(),
        self.fullscreen.clone(),
        self.picture_in_picture.clone(),
      );
      let buffered = (position.get() as f64 + 40.0).min(LENGTH);
      move || {
        let mut player = VideoPlayer::new()
          .media(picture())
          .width(width)
          .height(width * 9.0 / 16.0)
          .live(live)
          .buffered(buffered)
          .waiting(waiting)
          .ended(ended)
          .controls(controls)
          .playing(playing.clone())
          .position(position.clone())
          .volume(volume.clone())
          .muted(muted.clone())
          .rate(rate.clone())
          .captions(captions.clone())
          .caption_text(caption)
          .captions_track("English")
          .qualities(&["1080p", "720p", "480p"])
          .quality(quality.clone())
          .fullscreen(fullscreen.clone())
          .picture_in_picture(pip.clone())
          // The frame under the pointer of the seek bar: the picture stands for it.
          .preview(|_| picture().into());
        if !live {
          player = player.duration(LENGTH);
        }
        if let Some(size) = size {
          player = player.size(size);
        }
        if poster_on {
          player = player.poster(poster());
        }
        if error {
          player = player
            .error("This video can't be played")
            .error_description("Check the connection and try again.");
        }
        if overlay {
          player = player.child(
            Row::new()
              .padding(12.0)
              .child(text("Your layer", 12.0, FontWeight::Medium).color(Color::new(255, 255, 255, 255))),
          );
        }
        player
      }
    };
    let record = self.playground.recorder();
    let shown = {
      let make = make.clone();
      let (ended, position, error) = (self.ended.clone(), self.position.clone(), self.error.clone());
      let forced = self.playground.forced();
      move |ctx: &mut Ctx| {
        let say = |name: &'static str| {
          let record = record.clone();
          move |value: String| record(format!("{name}({value})"))
        };
        let (p, s, e, v, m, r, c, q, f, i, t) = (
          say("on_playing_change"),
          say("on_position_change"),
          say("on_position_change_end"),
          say("on_volume_change"),
          say("on_muted_change"),
          say("on_rate_change"),
          say("on_captions_change"),
          say("on_quality_change"),
          say("on_fullscreen_change"),
          say("on_picture_in_picture_change"),
          say("on_retry"),
        );
        let (replayed, seeked, retried) = ((ended.clone(), position.clone()), ended.clone(), error.clone());
        make()
          .force(forced)
          .on_playing_change(move |to| {
            // Played again once it ended: from the start.
            if to && replayed.0.get_untracked() {
              replayed.0.set(false);
              replayed.1.set(0.0);
            }
            p(to.to_string())
          })
          .on_position_change(move |to| {
            if to < LENGTH && seeked.get_untracked() {
              seeked.set(false);
            }
            s(format!("{to:.1}"))
          })
          .on_position_change_end(move |to| e(format!("{to:.1}")))
          .on_volume_change(move |to| v(format!("{to:.2}")))
          .on_muted_change(move |to| m(to.to_string()))
          .on_rate_change(move |to| r(to.to_string()))
          .on_captions_change(move |to| c(to.to_string()))
          .on_quality_change(move |to| q(format!("{to:?}")))
          .on_fullscreen_change(move |to| f(to.to_string()))
          .on_picture_in_picture_change(move |to| i(to.to_string()))
          .on_retry(move || {
            retried.set(false);
            t(String::new())
          })
          .key("played")
          .mount(ctx)
      }
    };

    let mut code = vec![
      "VideoPlayer::new()".to_owned(),
      "    .media(frame)".to_owned(),
      format!("    .width({width}.0)"),
      format!("    .height({}.0)", width * 9.0 / 16.0),
    ];
    if let Some(size) = size {
      code.push(format!("    .size(VideoPlayerSize::{size:?})"));
    }
    if !live {
      code.push(format!("    .duration({LENGTH:.1})"));
    }
    for (on, line) in [
      (live, "    .live(true)"),
      (waiting, "    .waiting(true)"),
      (ended, "    .ended(true)"),
      (!controls, "    .controls(false)"),
      (poster_on, "    .poster(poster)"),
      (error, "    .error(\"This video can't be played\")"),
      (overlay, "    .child(your_layer)"),
    ] {
      if on {
        code.push(line.to_owned());
      }
    }
    code.extend(
      [
        "    .playing(playing)",
        "    .position(position)",
        "    .volume(volume)",
        "    .muted(muted)",
        "    .rate(rate)",
        "    .captions(captions)",
        "    .caption_text(cue)",
        "    .qualities(&[\"1080p\", \"720p\", \"480p\"])",
        "    .quality(quality)",
        "    .fullscreen(fullscreen)",
        "    .picture_in_picture(pip)",
        "    .preview(|at| frame_at(at))",
        "    .mount(ctx)",
      ]
      .map(str::to_owned),
    );
    let tree = vec![
      format!("root        {} · radius lg, clipped", self.playground.measured()),
      "├─ media     the frame the app gives, the poster over it".to_owned(),
      "├─ veil      #00000099 while ended or failed".to_owned(),
      "├─ overlay   the app's own layer, with and without controls".to_owned(),
      "├─ center    the 64 disc · a spinner · the error".to_owned(),
      "├─ captions  16/500 on #000000B3, 12 over the bar".to_owned(),
      "└─ controls  88 · seek bar · row of 36 on a gradient to #000000B3".to_owned(),
    ];
    let facts = format!(
      "{} / {} · {}× · volume {:.0}%",
      media_time_text(position, false),
      if live {
        "live".to_owned()
      } else {
        media_time_text(LENGTH, false)
      },
      self.rate.get(),
      self.volume.get() * 100.0
    );
    let (set_size, set_width) = (self.size.clone(), self.width.clone());
    let reset = (
      [self.size.clone(), self.width.clone()],
      [
        self.playing.clone(),
        self.muted.clone(),
        self.captions.clone(),
        self.fullscreen.clone(),
        self.picture_in_picture.clone(),
        self.waiting.clone(),
        self.ended.clone(),
        self.live.clone(),
        self.error.clone(),
        self.overlay.clone(),
      ],
      [self.poster.clone(), self.controls.clone()],
      (
        self.position.clone(),
        self.volume.clone(),
        self.rate.clone(),
        self.quality.clone(),
      ),
    );
    let flags = [
      flag("playing", &self.playing),
      flag("waiting", &self.waiting),
      flag("ended", &self.ended),
      flag("live", &self.live),
      flag("error", &self.error),
      flag("poster", &self.poster),
      flag("muted", &self.muted),
      flag("captions", &self.captions),
      flag("fullscreen", &self.fullscreen),
      flag("picture_in_picture", &self.picture_in_picture),
      flag("controls", &self.controls),
      flag("child: your layer", &self.overlay),
    ];
    let mut controls_list = vec![
      prop(
        "size",
        "VideoPlayerSize",
        self.playground.select(0, &SIZES, &size, move |size| {
          set_size.set(SIZES.iter().position(|(_, chosen)| *chosen == size).unwrap_or(0))
        }),
      ),
      prop(
        "width",
        "Length",
        self.playground.select(1, &WIDTHS, &width, move |width| {
          set_width.set(WIDTHS.iter().position(|(_, chosen)| *chosen == width).unwrap_or(0))
        }),
      ),
      divider(),
    ];
    controls_list.extend(flags);
    self.playground.card(
      ctx,
      Spec {
        title: "VideoPlayer",
        make: Arc::new(move || -> Element { make().into() }),
        shown: Shown::built(shown),
        fill: false,
        forceable: true,
        facts,
        event: "on_playing_change / on_position_change / on_volume_change / …",
        code,
        tree,
        controls: controls_list,
        changed: [
          size.is_some(),
          width != 640.0,
          self.playing.get(),
          waiting,
          ended,
          live,
          error,
          !poster_on,
          self.muted.get(),
          self.captions.get(),
          self.fullscreen.get(),
          self.picture_in_picture.get(),
          !controls,
          overlay,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          for select in &reset.0 {
            select.set(0);
          }
          for flag in &reset.1 {
            flag.set(false);
          }
          for flag in &reset.2 {
            flag.set(true);
          }
          reset.3.0.set(0.0);
          reset.3.1.set(0.8);
          reset.3.2.set(1.0);
          reset.3.3.set(None);
        }),
      },
    )
  }

  /// A round button of the feed's column: a glyph on a dark disc, and a count under it.
  fn feed_action(glyph: &'static str, label: &str, count: &str) -> Column {
    Column::new()
      .spacing(4.0)
      .align_items(Alignment::Center)
      .child(
        Button::new(label)
          .icon(glyph)
          .icon_only(true)
          .shape(ButtonShape::Pill)
          .variant(ButtonVariant::Ghost)
          .style(ButtonPart::Root, |s| {
            s.square(40.0)
              .background(Color::new(0, 0, 0, 0x66))
              .foreground(Color::new(255, 255, 255, 255))
          }),
      )
      .child(text(count, 11.0, FontWeight::Medium).color(Color::new(255, 255, 255, 255)))
  }

  /// The board's vertical example: a player with no controls, and the app's own over its picture, made of the
  /// system's parts.
  fn feed(&self) -> VideoPlayer {
    let (playing, muted) = (self.feed_playing.get(), self.feed_muted.get());
    let (play, mute) = (self.feed_playing.clone(), self.feed_muted.clone());
    let white = Color::new(255, 255, 255, 255);
    let center = Row::new()
      .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .child(
        PlayButton::new()
          .variant(PlayButtonVariant::Overlay)
          .size(lurq::ui::PlayButtonSize::Xl)
          .playing(playing)
          .on_change(move |to| play.set(to)),
      );
    let top = Row::new().width(FILL).padding(12.0).justify(Justify::End).child(
      Button::new(if muted { "Unmute" } else { "Mute" })
        .icon(if muted { icons::VOLUME_OFF } else { icons::VOLUME })
        .icon_only(true)
        .shape(ButtonShape::Pill)
        .variant(ButtonVariant::Ghost)
        .style(ButtonPart::Root, |s| {
          s.square(32.0).background(Color::new(0, 0, 0, 0x66)).foreground(white)
        })
        .on_click(move |_| mute.update(|muted| *muted = !*muted)),
    );
    let shade = Gradient::linear(
      180.0,
      [
        GradientStop::at(Color::new(0, 0, 0, 0), 0.0),
        GradientStop::at(Color::new(0, 0, 0, 0x99), 1.0),
      ],
    );
    let bottom = Row::new()
      .width(FILL)
      .padding(12.0)
      .spacing(12.0)
      .align_items(Alignment::End)
      .background_gradient(shade)
      .child(
        Column::new()
          .flex(1.0)
          .min_width(0.0)
          .spacing(4.0)
          .child(text("@lurq.dev", 14.0, FontWeight::SemiBold).color(white))
          .child(
            text(
              "Frosted glass over a moving picture, drawn by the GPU.",
              13.0,
              FontWeight::Normal,
            )
            .color(white),
          ),
      )
      .child(
        Column::new()
          .spacing(12.0)
          .child(Self::feed_action(icons::HEART, "Like", "12.4K"))
          .child(Self::feed_action(icons::MESSAGE, "Comments", "318"))
          .child(Self::feed_action(icons::SHARE, "Share", "Share")),
      );
    let progress = Row::new()
      .width(FILL)
      .height(2.0)
      .background(Color::new(255, 255, 255, 0x33))
      .child(Row::new().width(Dimension::Pct(34.0)).height(2.0).background(white));
    // The play button is the top of the layer: the shade behind the words reaches up under it, and would take
    // its presses from above. What holds it is as large as the picture and is nothing to the pointer itself, so
    // the buttons at the two ends are pressed through it.
    let layer = Stack::new()
      .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
      .child(
        Column::new()
          .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
          .justify(Justify::SpaceBetween)
          .child(top)
          .child(Column::new().width(FILL).child(bottom).child(progress)),
      )
      .child(center.hit_test(HitTestBehavior::ContentOnly));
    VideoPlayer::new()
      .media(poster())
      .width(243.0)
      .height(432.0)
      .controls(false)
      .playing_controlled(playing, |_| {})
      .muted_controlled(muted, |_| {})
      .child(layer)
  }
}

impl Component for VideoPlayerPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let (playing, position, rate) = (ctx.signal(false), ctx.signal(0.0_f32), ctx.signal(1.0_f32));
    let (waiting, ended, error) = (ctx.signal(false), ctx.signal(false), ctx.signal(false));
    // The page's clock: while it plays it moves the position on by the time gone at the speed chosen, and at the
    // end it stops and says it ended.
    let clock = ctx.create_interval(TICK, {
      let (playing, position, rate) = (playing.clone(), position.clone(), rate.clone());
      let (waiting, ended, error) = (waiting.clone(), ended.clone(), error.clone());
      move || {
        if !playing.get_untracked() || waiting.get_untracked() || error.get_untracked() {
          return;
        }
        let next = position.get_untracked() + TICK.as_secs_f32() * rate.get_untracked();
        if f64::from(next) >= LENGTH {
          position.set(LENGTH as f32);
          playing.set(false);
          ended.set(true);
        } else {
          position.set(next);
        }
      }
    });
    clock.start();
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      size: ctx.signal(0),
      width: ctx.signal(0),
      playing,
      position,
      volume: ctx.signal(0.8),
      muted: ctx.signal(false),
      rate,
      captions: ctx.signal(false),
      quality: ctx.signal(None),
      fullscreen: ctx.signal(false),
      picture_in_picture: ctx.signal(false),
      waiting,
      ended,
      live: ctx.signal(false),
      error,
      poster: ctx.signal(true),
      controls: ctx.signal(true),
      overlay: ctx.signal(false),
      feed_playing: ctx.signal(false),
      feed_muted: ctx.signal(true),
      _clock: clock,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let playground = self.play(ctx);
    let states = items()
      .child(item(
        state().position_controlled(0.0, |_| {}).poster(poster()),
        "idle · the poster and its length",
      ))
      .child(item(
        state().playing_controlled(true, |_| {}),
        "playing · the bar shown",
      ))
      .child(item(
        state()
          .playing_controlled(true, |_| {})
          .force_controls_shown(Some(false)),
        "playing · the bar faded after 3 s",
      ))
      .child(item(state(), "paused · the bar stays"))
      .child(item(state().playing_controlled(true, |_| {}).waiting(true), "waiting"))
      .child(item(
        state().ended(true).position_controlled(LENGTH, |_| {}),
        "ended · replay",
      ))
      .child(item(
        state()
          .error("This video can't be played")
          .error_description("Check the connection and try again.")
          .on_retry(|| {}),
        "error",
      ))
      .child(item(
        VideoPlayer::new()
          .media(picture())
          .width(432.0)
          .height(243.0)
          .live(true)
          .playing_controlled(true, |_| {}),
        "live · no duration, the bar at the edge",
      ))
      .child(item(
        state()
          .captions_controlled(true, |_| {})
          .caption_text("The bar fades out while the video plays."),
        "captions · above the bar",
      ))
      .child(item(state().muted_controlled(true, |_| {}), "muted"))
      .child(item(
        state().fullscreen_controlled(true, |_| {}),
        "fullscreen · the glyph exits",
      ));
    let sizes = items()
      .child(item(state().width(640.0).height(360.0), "large · 640 × 360"))
      .child(item(
        state().width(320.0).height(180.0).size(VideoPlayerSize::Md),
        "compact · 320 × 180",
      ))
      .child(item(
        state().width(200.0).height(113.0).size(VideoPlayerSize::Sm),
        "minimal · 200 × 113",
      ));
    let bare = items().child(item(state().controls(false), "the picture alone, as in a feed"));
    let feed = self.feed().key("feed").mount(ctx);
    let own = items().child(item(feed, "your own controls in the overlay"));
    let rows = [
      (
        "media",
        "impl Into<Element>",
        "",
        "What is drawn as the picture, filling the player: the frame the app has.",
      ),
      (
        "poster",
        "impl Into<Element>",
        "",
        "Drawn over the picture before the first play, with the length at its end.",
      ),
      (
        "child / children",
        "impl Into<Element>",
        "",
        "The app's own layer over the whole picture, with the controls and without them.",
      ),
      (
        "controls",
        "bool",
        "true",
        "The bar and the middle of the picture. Without them the player is its picture and the app's layer.",
      ),
      (
        "playing",
        "Signal<bool>",
        "",
        "Whether it plays. Also playing_controlled(bool, handler) and on_playing_change.",
      ),
      (
        "position",
        "Signal<f32>",
        "",
        "Where it is, in seconds. Also position_controlled; on_position_change as the handle moves, on_position_change_end where it is let go.",
      ),
      (
        "duration",
        "f64",
        "",
        "How long it is. Until said, the time reads -:-- / -:--.",
      ),
      ("buffered", "f64", "", "How far it is loaded, drawn in the seek bar."),
      (
        "preview",
        "Fn(f64) -> Element",
        "",
        "The frame for a time, shown over the time under the pointer of the seek bar (mounted).",
      ),
      (
        "volume",
        "Signal<f32>",
        "",
        "How loud, from 0 to 1. Also volume_controlled, on_volume_change, on_volume_change_end.",
      ),
      (
        "muted",
        "Signal<bool>",
        "",
        "Whether the sound is off; the volume is kept. Also muted_controlled and on_muted_change.",
      ),
      (
        "rate / rates",
        "Signal<f32>, &[f32]",
        "1, MEDIA_RATES",
        "How fast it plays, and the speeds offered. Also rate_controlled and on_rate_change.",
      ),
      (
        "captions",
        "Signal<bool>",
        "",
        "Whether the captions show. Also captions_controlled and on_captions_change.",
      ),
      (
        "caption_text / captions_track",
        "&str",
        "",
        "The line to show now, and the track's name the settings say.",
      ),
      (
        "qualities / quality",
        "&[&str], Signal<Option<usize>>",
        "None",
        "The qualities offered after Auto, and the one it plays in. Also quality_controlled and on_quality_change.",
      ),
      (
        "fullscreen",
        "Signal<bool>",
        "",
        "Whether it fills the screen; its button then exits. Also fullscreen_controlled and on_fullscreen_change.",
      ),
      (
        "picture_in_picture",
        "Signal<bool>",
        "",
        "Whether it plays in a small window of its own. Also its _controlled form and handler.",
      ),
      (
        "waiting",
        "bool",
        "false",
        "It waits for what it plays: a spinner in the middle.",
      ),
      (
        "ended",
        "bool",
        "false",
        "It played to the end: the veil and replay. A press of replay tells playing true.",
      ),
      (
        "live",
        "bool",
        "false",
        "No end: the LIVE badge for the time, the bar full, no speed.",
      ),
      (
        "error / error_description",
        "&str",
        "",
        "That it cannot play: the veil, the title and the line in the middle, and no bar.",
      ),
      (
        "on_retry",
        "impl Fn()",
        "",
        "The error's button, drawn once this is given.",
      ),
      (
        "size",
        "VideoPlayerSize",
        "from the width",
        "Lg, Md or Sm in place of the form its width gives it (Md under 400, Sm under 280).",
      ),
      (
        "width / height",
        "Length",
        "640 × 360",
        "Its size, in place of its form's.",
      ),
      (
        "force_menu / force_controls_shown",
        "Option<..>",
        "None",
        "A menu drawn open, or the bar shown or faded, with nothing pointing at it.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Keeps its width, the timer that fades the bar, its menus and its tooltips.",
      ),
    ];
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Video Player"],
        title: "Video Player",
        description: "A picture the app gives, the controls over it, and a layer of the app's own: an ordinary \
                      player, or only the picture as in a vertical feed.",
        import: Some("use lurq::ui::VideoPlayer;"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of VideoPlayer is a control on the right. Nothing is decoded: a clock of the page's own moves \
           the position on while it plays, and at the end it stops and says it ended. Every press of the player \
           tells the page what it is to be, and the page sets it.",
          playground,
        ),
        Section::new(
          "States",
          "Paused the play disc stands in the middle and the bar stays; playing, the bar fades out 3 s after the \
           pointer rests. Ended and failed are under a veil. Live has its badge for the time.",
          self.frame.demo(
            0,
            states,
            ".ended(true)",
            &[
              "VideoPlayer::new().media(frame).duration(250.0).position(position)",
              "    .ended(true)",
              "",
              "VideoPlayer::new().media(frame)",
              "    .error(\"This video can't be played\")",
              "    .error_description(\"Check the connection and try again.\")",
              "    .on_retry(move || retry())",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "The form follows the player's width: compact under 400, minimal under 280. The large form folds its \
           volume into its button under 480. size says the form where the app wants another.",
          self.frame.demo(
            1,
            sizes,
            ".size(VideoPlayerSize::Md)",
            &[
              "VideoPlayer::new().media(frame).width(640.0).height(360.0)",
              "VideoPlayer::new().media(frame).width(320.0).height(180.0)",
              "VideoPlayer::new().media(frame).width(200.0).height(113.0)",
            ],
          ),
        ),
        Section::new(
          "Without controls",
          "The picture and the app's layer alone, with no bar and nothing in the middle.",
          self.frame.demo(
            2,
            bare,
            ".controls(false)",
            &["VideoPlayer::new().media(frame).controls(false)"],
          ),
        ),
        Section::new(
          "Your own controls",
          "With no controls of its own, the player holds the app's over its picture: here made of the system's \
           parts, the overlay play button and round buttons on dark discs.",
          self.frame.demo(
            3,
            own,
            ".controls(false).child(layer)",
            &[
              "VideoPlayer::new()",
              "    .media(frame)",
              "    .width(243.0)",
              "    .height(432.0)",
              "    .controls(false)",
              "    .child(",
              "        Stack::new()",
              "            .child(PlayButton::new().variant(PlayButtonVariant::Overlay).on_change(play))",
              "            .child(your_column_of_actions),",
              "    )",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new("Props", "", props_table(&rows, &[], self.frame.room())),
      ],
    )
  }
}
