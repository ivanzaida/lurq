//! The page of the audio queue: its playground, the board's rows and their states, and its props.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::{Element, gradient::Gradient},
  ui::{AudioQueue, AudioQueueItem, Interaction},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag},
};

/// The board's four tracks: title, who by, length, and the colors of their artwork.
const TRACKS: [(&str, &str, f64, [PaletteColor; 2]); 4] = [
  (
    "Night Drive",
    "Aurora Lane",
    250.0,
    [PaletteColor::Chart4, PaletteColor::Chart1],
  ),
  (
    "Paper Boats",
    "Mira Sol",
    232.0,
    [PaletteColor::Chart2, PaletteColor::Chart3],
  ),
  (
    "Glass Garden",
    "The Fernweh",
    301.0,
    [PaletteColor::Chart5, PaletteColor::Chart2],
  ),
  (
    "Low Tide",
    "Aurora Lane",
    198.0,
    [PaletteColor::Chart1, PaletteColor::Chart5],
  ),
];

/// An artwork of 32: a gradient of two of the board's chart colors.
fn artwork(colors: [PaletteColor; 2]) -> Row {
  Row::new()
    .width(32.0)
    .height(32.0)
    .background_gradient(Gradient::linear(135.0, colors))
}

/// The board's tracks as rows: with artwork or not, the third unavailable or not, each "more" told to `more`.
fn tracks(media: bool, unavailable: bool, more: Option<Arc<dyn Fn(usize) + Send + Sync>>) -> Vec<AudioQueueItem> {
  TRACKS
    .iter()
    .enumerate()
    .map(|(index, (title, by, seconds, colors))| {
      let mut item = AudioQueueItem::new(title)
        .description(by)
        .duration(*seconds)
        .disabled(unavailable && index == 2);
      if media {
        item = item.media(artwork(colors.clone()));
      }
      if let Some(more) = &more {
        let more = more.clone();
        item = item.on_more(move || more(index));
      }
      item
    })
    .collect()
}

/// A queue on the page: 360 wide, as the board draws it.
fn held(queue: impl Into<Element>) -> Column {
  Column::new()
    .width(360.0)
    .max_width(FILL)
    .overflow_visible()
    .child(queue)
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .overflow_visible()
    .child(shown)
    .child(quiet(caption, 12.0))
}

pub struct AudioQueuePage {
  frame: Frame,
  playground: Playground,
  current: Signal<Option<usize>>,
  playing: Signal<bool>,
  media: Signal<bool>,
  unavailable: Signal<bool>,
  more: Signal<bool>,
  disabled: Signal<bool>,
}

impl AudioQueuePage {
  /// The playground: the queue as the panel's props make it, bound to the page's current track.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (playing, media, unavailable, more, disabled) = (
      self.playing.get(),
      self.media.get(),
      self.unavailable.get(),
      self.more.get(),
      self.disabled.get(),
    );
    let current = self.current.get();
    let record = self.playground.recorder();
    let told: Option<Arc<dyn Fn(usize) + Send + Sync>> = more.then(|| {
      let record = record.clone();
      Arc::new(move |index: usize| record(format!("items[{index}].on_more()"))) as Arc<dyn Fn(usize) + Send + Sync>
    });
    let plain = move || {
      AudioQueue::controlled(current, tracks(media, unavailable, None), |_| {})
        .playing(playing)
        .disabled(disabled)
    };
    let queue = AudioQueue::new(self.current.clone(), tracks(media, unavailable, told))
      .playing(playing)
      .disabled(disabled)
      .force(self.playground.forced())
      .on_change(move |index| record(format!("on_change({index})")));

    let mut code = vec![
      "// current: Signal<Option<usize>>".to_owned(),
      "AudioQueue::new(current, [".to_owned(),
      "    AudioQueueItem::new(\"Night Drive\").description(\"Aurora Lane\").duration(250.0),".to_owned(),
      "    AudioQueueItem::new(\"Paper Boats\").description(\"Mira Sol\").duration(232.0),".to_owned(),
      "    // ...".to_owned(),
      "])".to_owned(),
    ];
    for (name, on) in [("playing", playing), ("disabled", disabled)] {
      if on {
        code.push(format!("    .{name}(true)"));
      }
    }
    code.push("    .mount(ctx)".to_owned());
    let mut tree = vec![format!("root          {}", self.playground.measured())];
    for (index, (title, ..)) in TRACKS.iter().enumerate() {
      let branch = if index + 1 == TRACKS.len() { "└─" } else { "├─" };
      let state = if current == Some(index) {
        if playing { "  playing" } else { "  current" }
      } else if unavailable && index == 2 {
        "  unavailable"
      } else {
        ""
      };
      tree.push(format!("{branch} item 48   {} {title}{state}", index + 1));
    }

    let flags = [
      self.playing.clone(),
      self.media.clone(),
      self.unavailable.clone(),
      self.more.clone(),
      self.disabled.clone(),
    ];
    let reset = {
      let (current, flags) = (self.current.clone(), flags.clone());
      move || {
        current.set(Some(1));
        for (index, flag) in flags.iter().enumerate() {
          // Playing, the artwork, an unavailable track and the more buttons are what the board draws.
          flag.set(index < 4);
        }
      }
    };
    let changed = usize::from(current != Some(1))
      + [!playing, !media, !unavailable, !more, disabled]
        .into_iter()
        .filter(|changed| *changed)
        .count();
    let playground = &self.playground;
    self.playground.card(
      ctx,
      Spec {
        title: "Audio Queue",
        make: Arc::new(move || held(plain()).into()),
        shown: Shown::built(move |ctx| held(playground.measure(queue.mount(ctx)).width(FILL)).into()).measured(),
        fill: false,
        forceable: true,
        facts: match current {
          Some(index) => format!("on track {}", index + 1),
          None => "on no track".to_owned(),
        },
        event: "on_change",
        code,
        tree,
        controls: vec![
          divider(),
          flag("playing", &self.playing),
          flag("media", &self.media),
          flag("unavailable (third)", &self.unavailable),
          flag("on_more", &self.more),
          flag("disabled", &self.disabled),
        ],
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for AudioQueuePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      current: ctx.signal(Some(1)),
      playing: ctx.signal(true),
      media: ctx.signal(true),
      unavailable: ctx.signal(true),
      more: ctx.signal(true),
      disabled: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = held(
      AudioQueue::controlled(Some(1), tracks(true, true, Some(Arc::new(|_| {}))), |_| {})
        .playing(true)
        .mount(ctx),
    );
    let states = Row::new()
      .width(FILL)
      .spacing(28.0)
      .wrap()
      .align_items(Alignment::Start)
      .overflow_visible()
      .child(item(
        held(
          AudioQueue::controlled(None, tracks(true, false, None).into_iter().take(2), |_| {})
            .force(Some(Interaction::Hovered)),
        ),
        "hover on the first",
      ))
      .child(item(
        held(
          AudioQueue::controlled(None, tracks(true, false, None).into_iter().take(2), |_| {})
            .force(Some(Interaction::FocusVisible)),
        ),
        "focus on the first",
      ))
      .child(item(
        held(AudioQueue::controlled(Some(0), tracks(false, false, None).into_iter().take(2), |_| {}).playing(false)),
        "current, paused; no artwork",
      ));
    let rows = [
      (
        "new",
        "Signal<Option<usize>>, impl IntoIterator<Item = AudioQueueItem>",
        "",
        "The tracks, on the one whose index the signal holds, which a press of a row sets.",
      ),
      (
        "controlled",
        "Option<usize>, items, impl Fn(usize)",
        "",
        "In place of new: the current track held by the app, which is told the index of a row pressed.",
      ),
      (
        "on_change",
        "impl Fn(usize)",
        "",
        "The index of a row pressed, where it is another than the current one and can be played.",
      ),
      (
        "playing",
        "bool",
        "false",
        "Whether the current track plays: the equalizer stands for its number.",
      ),
      ("item / items", "AudioQueueItem", "", "Adds tracks at the end."),
      (
        "AudioQueueItem::new",
        "&str",
        "",
        "A track by its title; description, media, duration (seconds), disabled, on_select, on_more, more_anchor.",
      ),
      (
        "label / more_label / playing_label",
        "&str",
        "",
        "What the queue, a row's more button and the equalizer are called.",
      ),
      ("disabled", "bool", "false", "Half the opacity; it takes nothing."),
      (
        "mount",
        "&mut Ctx",
        "",
        "The tooltips of the more buttons. Converted into an element, the buttons have none.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Audio Queue"],
        title: "Audio Queue",
        description: "The tracks a player plays one after another, the one it is on marked. The arrow keys move \
                      between the rows, Enter plays one.",
        import: Some("use lurq::ui::{AudioQueue, AudioQueueItem};"),
        status: Some("New"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The queue on the stage is bound to the page's current track: a press of a row makes it the current one. \
           Tab enters at it; the arrows move between the rows, past one that cannot be played.",
          playground,
        ),
        Section::new(
          "Queue",
          "The board's four rows: the second plays, with the accent fill and the equalizer; the third cannot be \
           played. Under the pointer a row shows the play glyph for its number and its more button.",
          self.frame.demo(
            0,
            board,
            "AudioQueue::new(current, items).playing(true)",
            &[
              "AudioQueue::new(current, tracks)",
              "    .playing(true)",
              "    .on_change(|index| play(index))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "States",
          "A row under the pointer, focused from the keyboard, and the current track while it is paused.",
          self.frame.demo(
            1,
            states,
            "AudioQueue::controlled(..).force(Some(Interaction::Hovered))",
            &["AudioQueue::controlled(None, tracks, |index| play(index))"],
          ),
        ),
        Section::new("Props", "", props_table(&rows, &["new"], self.frame.room())),
      ],
    )
  }
}
