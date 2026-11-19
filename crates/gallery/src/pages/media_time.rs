//! The page of a player's time: its playground, counted by a clock of the page's own while it plays; its forms,
//! over a picture, and its sizes.

use std::{sync::Arc, time::Duration};

use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, Interval},
  },
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{MediaTime, MediaTimeSize, media_time_text},
};

use super::seek_bar::picture;
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(10.0)
    .child(Row::new().max_width(FILL).child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

/// How long what the playground plays is; none where it is not known yet.
const DURATIONS: [(&str, Option<f64>); 3] = [("4:10", Some(250.0)), ("2:10:00", Some(7800.0)), ("Not loaded", None)];
const SIZES: [(&str, MediaTimeSize); 3] = [
  ("Sm · 11", MediaTimeSize::Sm),
  ("Md · 12", MediaTimeSize::Md),
  ("Lg · 13", MediaTimeSize::Lg),
];
const TICK: Duration = Duration::from_millis(250);

pub struct MediaTimePage {
  frame: Frame,
  playground: Playground,
  position: Signal<f64>,
  playing: Signal<bool>,
  duration: Signal<Option<f64>>,
  size: Signal<MediaTimeSize>,
  label: Signal<String>,
  remaining: Signal<bool>,
  pressable: Signal<bool>,
  on_media: Signal<bool>,
  live: Signal<bool>,
  _clock: Interval,
}

impl MediaTimePage {
  /// The playground: the time as the panel's props make it, at the position the page's clock counts.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (position, duration, size, label) = (
      self.position.get(),
      self.duration.get(),
      self.size.get(),
      self.label.get(),
    );
    let (remaining, pressable, on_media, live) = (
      self.remaining.get(),
      self.pressable.get(),
      self.on_media.get(),
      self.live.get(),
    );
    let plain = {
      let label = label.clone();
      move || {
        let time = MediaTime::new(position, duration)
          .remaining(remaining)
          .on_media(on_media)
          .live(live)
          .size(size);
        if label.is_empty() { time } else { time.label(&label) }
      }
    };
    let mut time = self.playground.styled(plain()).force(self.playground.forced());
    if pressable {
      let (record, turn) = (self.playground.recorder(), self.remaining.clone());
      time = time.on_click(move |_| {
        record("on_click".to_owned());
        turn.set(!turn.get_untracked());
      });
    }

    let mut code = vec![match duration {
      Some(duration) => format!("MediaTime::new(position, Some({duration:?}))"),
      None => "MediaTime::new(position, None)".to_owned(),
    }];
    if remaining {
      code.push("    .remaining(true)".to_owned());
    }
    if on_media {
      code.push("    .on_media(true)".to_owned());
    }
    if live {
      code.push("    .live(true)".to_owned());
    }
    match size {
      MediaTimeSize::Sm => code.push("    .size(MediaTimeSize::Sm)".to_owned()),
      MediaTimeSize::Lg => code.push("    .size(MediaTimeSize::Lg)".to_owned()),
      MediaTimeSize::Md => {}
    }
    if !label.is_empty() {
      code.push(format!("    .label({label:?})"));
    }
    if pressable {
      code.push("    .on_click(|_| remaining.set(!remaining.get()))".to_owned());
    }

    let hours = duration.is_some_and(|duration| duration >= 3600.0);
    let tree = if live {
      vec![
        format!("root        {}", self.playground.measured()),
        "└─ badge    \"LIVE\", destructive".to_owned(),
      ]
    } else {
      match (duration, remaining) {
        (None, _) => vec![
          format!("root        {}", self.playground.measured()),
          "├─ elapsed  \"-:--\"".to_owned(),
          "├─ divider  \"/\"".to_owned(),
          "└─ total    \"-:--\"".to_owned(),
        ],
        (Some(duration), true) => vec![
          format!("root        {}", self.playground.measured()),
          format!(
            "└─ elapsed  \"-{}\"",
            media_time_text((duration - position).max(0.0), hours)
          ),
        ],
        (Some(duration), false) => vec![
          format!("root        {}", self.playground.measured()),
          format!("├─ elapsed  \"{}\"", media_time_text(position, hours)),
          "├─ divider  \"/\"".to_owned(),
          format!("└─ total    \"{}\"", media_time_text(duration, hours)),
        ],
      }
    };

    let (to_position, to_playing, to_duration, to_size, to_label) = (
      self.position.clone(),
      self.playing.clone(),
      self.duration.clone(),
      self.size.clone(),
      self.label.clone(),
    );
    let flags = [
      self.remaining.clone(),
      self.pressable.clone(),
      self.on_media.clone(),
      self.live.clone(),
    ];
    let (set_duration, set_size) = (self.duration.clone(), self.size.clone());
    let shown = if on_media {
      let measured = self.playground.measure(time);
      Shown::of(Column::new().width(280.0).child(picture(measured, 72.0))).measured()
    } else {
      Shown::of(time)
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Media Time",
        make: Arc::new(move || plain().into()),
        shown,
        fill: false,
        forceable: pressable,
        facts: "mono".to_owned(),
        event: if pressable { "on_click" } else { "" },
        code,
        tree,
        controls: vec![
          flag("playing (the page's clock)", &self.playing),
          divider(),
          prop(
            "duration",
            "Option<f64>",
            self
              .playground
              .select(0, &DURATIONS, &duration, move |next| set_duration.set(next)),
          ),
          prop(
            "size",
            "MediaTimeSize",
            self.playground.select(1, &SIZES, &size, move |next| set_size.set(next)),
          ),
          prop("label", "&str", field(&self.label)),
          divider(),
          flag("remaining", &self.remaining),
          flag("on_click (turns remaining)", &self.pressable),
          flag("on_media", &self.on_media),
          flag("live", &self.live),
        ],
        changed: [
          duration != Some(250.0),
          size != MediaTimeSize::Md,
          !label.is_empty(),
          remaining,
          pressable,
          on_media,
          live,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_playing.set(false);
          to_position.set(84.0);
          to_duration.set(Some(250.0));
          to_size.set(MediaTimeSize::Md);
          to_label.set(String::new());
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for MediaTimePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let (position, playing, duration) = (ctx.signal(84.0_f64), ctx.signal(false), ctx.signal(Some(250.0_f64)));
    // The page's clock counts while it plays, to the end.
    let clock = ctx.create_interval(TICK, {
      let (position, playing, duration) = (position.clone(), playing.clone(), duration.clone());
      move || {
        let whole = duration.get_untracked().unwrap_or(f64::INFINITY);
        let next = (position.get_untracked() + TICK.as_secs_f64()).min(whole);
        position.set(next);
        if next >= whole {
          playing.set(false);
        }
      }
    });
    clock.stop();
    {
      let clock = clock.clone();
      ctx.watch(&playing, move |on| if *on { clock.start() } else { clock.stop() });
    }
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      position,
      playing,
      duration,
      size: ctx.signal(MediaTimeSize::Md),
      label: ctx.signal(String::new()),
      remaining: ctx.signal(false),
      pressable: ctx.signal(false),
      on_media: ctx.signal(false),
      live: ctx.signal(false),
      _clock: clock,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let forms = items()
      .child(item(MediaTime::new(84.0, Some(250.0)), "time · elapsed / total"))
      .child(item(MediaTime::new(3765.0, Some(7800.0)), "over an hour"))
      .child(item(
        MediaTime::new(84.0, Some(250.0)).remaining(true),
        "remaining · on click",
      ))
      .child(item(MediaTime::new(0.0, None), "not loaded"))
      .child(item(MediaTime::new(0.0, None).live(true), "live · no duration"));
    let over = items()
      .child(item(
        Column::new()
          .width(220.0)
          .child(picture(MediaTime::new(84.0, Some(250.0)).on_media(true), 64.0)),
        "over a picture · white, the whole at 60%",
      ))
      .child(item(
        Column::new()
          .width(220.0)
          .child(picture(MediaTime::new(0.0, None).on_media(true), 64.0)),
        "not loaded, over a picture",
      ));
    let sizes = items()
      .child(item(
        MediaTime::new(84.0, Some(250.0)).size(MediaTimeSize::Sm),
        "sm · 11, a compact player",
      ))
      .child(item(MediaTime::new(84.0, Some(250.0)), "md · 12"))
      .child(item(
        MediaTime::new(84.0, Some(250.0)).size(MediaTimeSize::Lg),
        "lg · 13, a phone across",
      ));
    let rows = [
      ("position", "impl Into<f64>", "", "How far it is, in seconds."),
      (
        "duration",
        "Option<f64>",
        "",
        "How long the whole is, in seconds; None while it is not known: it reads -:-- / -:--. From an hour up both \
         read hours.",
      ),
      (
        "remaining",
        "bool",
        "false",
        "Reads what is left, with a minus before it, alone.",
      ),
      (
        "on_media",
        "bool",
        "false",
        "Over a picture: white, the / and the whole #FFFFFF99, in a light and a dark theme.",
      ),
      (
        "live",
        "bool",
        "false",
        "A live stream: a badge in the destructive color reads LIVE in place of the time.",
      ),
      (
        "live_text",
        "&str",
        "\"LIVE\"",
        "What the badge of a live stream reads.",
      ),
      (
        "size",
        "MediaTimeSize",
        "Md",
        "Sm is 11, a compact player's; Md 12; Lg 13, a phone's player held across.",
      ),
      (
        "on_click",
        "impl IntoMouseEventHandler",
        "",
        "A press: a player turns the reading between elapsed and remaining. Given it, the time is a button and a \
         stop of Tab. Each call adds.",
      ),
      (
        "label",
        "&str",
        "",
        "What it is called, to tooling that reads the tree: what it reads unless told (1:24 of 4:10).",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Media Time"],
        title: "Media Time",
        description: "The time a player reads out: how far it is, and how long the whole is.",
        import: Some("use lurq::ui::MediaTime;"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of MediaTime is a control on the right. Nothing plays: the page's own clock counts while \
           \"playing\" is on.",
          playground,
        ),
        Section::new(
          "Forms",
          "The mono type at 12, so that it does not jump as it counts: elapsed in the foreground, the / and the \
           whole muted. What is left reads alone; not loaded, it reads -:-- / -:--; live, a badge.",
          self.frame.demo(
            0,
            forms,
            "MediaTime::new(84.0, Some(250.0))",
            &[
              "MediaTime::new(84.0, Some(250.0))",
              "MediaTime::new(3765.0, Some(7800.0))",
              "MediaTime::new(84.0, Some(250.0)).remaining(true)",
              "MediaTime::new(0.0, None)",
              "MediaTime::new(0.0, None).live(true)",
            ],
          ),
        ),
        Section::new(
          "Over a picture",
          "White whatever the theme, the / and the whole at 60%: the picture is not themed.",
          self.frame.demo(
            1,
            over,
            ".on_media(true)",
            &[
              "MediaTime::new(84.0, Some(250.0)).on_media(true)",
              "MediaTime::new(0.0, None).on_media(true)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "A compact player's and an audio player's time is 11; a phone's player held across reads 13.",
          self.frame.demo(
            2,
            sizes,
            ".size(MediaTimeSize::Sm)",
            &[
              "MediaTime::new(84.0, Some(250.0)).size(MediaTimeSize::Sm)",
              "MediaTime::new(84.0, Some(250.0))",
              "MediaTime::new(84.0, Some(250.0)).size(MediaTimeSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a time is given. media_time_text(seconds, hours) is the reading alone, as a string.",
          props_table(&rows, &["position", "duration"], self.frame.room()),
        ),
      ],
    )
  }
}
