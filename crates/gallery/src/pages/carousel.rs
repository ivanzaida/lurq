//! The page of the carousel: its playground, the carousel as the board draws it, one of pictures, one that shows
//! several slides at once, and one that turns by itself.

use std::{sync::Arc, time::Duration};

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::Signal,
  images::ImageData,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::Element,
  ui::{Carousel, Picture},
};

use super::picture::lake;
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const COUNTS: [(&str, usize); 3] = [("4", 4), ("6", 6), ("10", 10)];
const IN_VIEW: [(&str, usize); 3] = [("1", 1), ("2", 2), ("3", 3)];
const WIDTHS: [(&str, f32); 3] = [("336", 336.0), ("560", 560.0), ("280", 280.0)];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  // No wider than the stage: what is shown gets narrower with it, and what is said under it wraps.
  Column::new()
    .max_width(FILL)
    .spacing(10.0)
    .child(Row::new().max_width(FILL).padding(2.0).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// A slide as the board draws one: a card 140 high with its number in the middle.
fn slide(number: usize) -> Column {
  Column::new()
    .height(140.0)
    .rounded(RadiusSize::Xl)
    .background(PaletteColor::Card)
    .foreground(PaletteColor::CardForeground)
    .border(line())
    .justify(Justify::Center)
    .align_items(Alignment::Center)
    .child(text(&number.to_string(), 36.0, FontWeight::SemiBold))
}

fn slides(count: usize) -> Vec<Column> {
  (1..=count).map(slide).collect()
}

/// The page's pictures, 16 by 9, turn by turn.
fn pictures(shots: &[ImageData; 3], count: usize) -> Vec<Picture> {
  (0..count)
    .map(|at| {
      Picture::new(shots[at % shots.len()].clone())
        .label("The lake at dusk")
        .aspect_ratio(16.0 / 9.0)
    })
    .collect()
}

pub struct CarouselPage {
  frame: Frame,
  playground: Playground,
  count: Signal<usize>,
  per_view: Signal<usize>,
  width: Signal<f32>,
  wrap: Signal<bool>,
  arrows: Signal<bool>,
  dots: Signal<bool>,
  autoplay: Signal<bool>,
  disabled: Signal<bool>,
  /// Which slide the playground's carousel is at, and the board's.
  index: Signal<usize>,
  board: Signal<usize>,
  /// The page's pictures, drawn once: the lake as it is, standing and lying.
  shots: [ImageData; 3],
}

impl CarouselPage {
  /// The playground: a carousel as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (count, per_view, width) = (self.count.get(), self.per_view.get(), self.width.get());
    let (wrap, arrows, dots) = (self.wrap.get(), self.arrows.get(), self.dots.get());
    let (autoplay, disabled) = (self.autoplay.get(), self.disabled.get());
    let record = self.playground.recorder();
    let index = self.index.clone();

    let plain = move || {
      let mut carousel = Carousel::new(slides(count))
        .per_view(per_view)
        .width(width)
        .wrap(wrap)
        .arrows(arrows)
        .dots(dots)
        .disabled(disabled);
      if autoplay {
        carousel = carousel.autoplay(Duration::from_secs(3));
      }
      carousel
    };
    let live = {
      let (plain, record, playground) = (plain.clone(), record.clone(), &self.playground);
      move |_: &mut Ctx| -> Element {
        let record = record.clone();
        playground
          .styled(
            plain()
              .index(index.clone())
              .on_index_change(move |to| record(format!("on_index_change({to})"))),
          )
          .into()
      }
    };
    let still = {
      let plain = plain.clone();
      move || -> Element { plain().into() }
    };

    let mut code = vec!["Carousel::new(slides)".to_owned()];
    if per_view != 1 {
      code.push(format!("    .per_view({per_view})"));
    }
    if wrap {
      code.push("    .wrap(true)".to_owned());
    }
    if !arrows {
      code.push("    .arrows(false)".to_owned());
    }
    if !dots {
      code.push("    .dots(false)".to_owned());
    }
    if autoplay {
      code.push("    .autoplay(Duration::from_secs(3))".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .index(shown)".to_owned());
    let mut tree = vec!["root".to_owned()];
    tree.push(
      if arrows {
        "├─ track      the button before, the slides, the button after"
      } else {
        "├─ track      the slides"
      }
      .to_owned(),
    );
    tree.push("│  └─ viewport  the slides in a row, one gap wider than it".to_owned());
    tree.push(
      if dots {
        "└─ dots       one for each place it can be at"
      } else {
        "└─ (no dots)"
      }
      .to_owned(),
    );

    let (to_count, to_per_view, to_width) = (self.count.clone(), self.per_view.clone(), self.width.clone());
    let (to_wrap, to_arrows, to_dots) = (self.wrap.clone(), self.arrows.clone(), self.dots.clone());
    let (to_autoplay, to_disabled, to_index) = (self.autoplay.clone(), self.disabled.clone(), self.index.clone());
    let (pick_count, pick_per_view, pick_width) = (self.count.clone(), self.per_view.clone(), self.width.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Carousel",
        make: Arc::new(still),
        shown: Shown::built(live),
        fill: false,
        forceable: false,
        facts: format!("{count} slides · {per_view} in view"),
        event: "on_index_change",
        code,
        tree,
        controls: vec![
          prop(
            "slides",
            "how many",
            self
              .playground
              .select(0, &COUNTS, &count, move |which| pick_count.set(which)),
          ),
          prop(
            "per_view",
            "usize",
            self
              .playground
              .select(1, &IN_VIEW, &per_view, move |which| pick_per_view.set(which)),
          ),
          prop(
            "width",
            "Length",
            self
              .playground
              .select(2, &WIDTHS, &width, move |which| pick_width.set(which)),
          ),
          divider(),
          flag("wrap", &self.wrap),
          flag("arrows", &self.arrows),
          flag("dots", &self.dots),
          flag("autoplay", &self.autoplay),
          flag("disabled", &self.disabled),
        ],
        changed: [
          count != 4,
          per_view != 1,
          width != 336.0,
          wrap,
          !arrows,
          !dots,
          autoplay,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_count.set(4);
          to_per_view.set(1);
          to_width.set(336.0);
          to_wrap.set(false);
          to_arrows.set(true);
          to_dots.set(true);
          to_autoplay.set(false);
          to_disabled.set(false);
          to_index.set(0);
        }),
      },
    )
  }
}

impl Component for CarouselPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      count: ctx.signal(4),
      per_view: ctx.signal(1),
      width: ctx.signal(336.0),
      wrap: ctx.signal(false),
      arrows: ctx.signal(true),
      dots: ctx.signal(true),
      autoplay: ctx.signal(false),
      disabled: ctx.signal(false),
      index: ctx.signal(0),
      board: ctx.signal(1),
      shots: [lake(640, 360), lake(360, 540), lake(900, 400)],
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = Row::new().width(FILL).id("carousel-board").child(item(
      Carousel::new(slides(4)).width(336.0).index(self.board.clone()),
      "four slides · at the second, as the board draws it",
    ));

    let photos = Row::new().width(FILL).id("carousel-pictures").child(item(
      Carousel::new(pictures(&self.shots, 6))
        .width(560.0)
        .wrap(true)
        .label("Photos"),
      "pictures · 16:9 · the first after the last",
    ));

    let several = Row::new()
      .width(FILL)
      .spacing(24.0)
      .wrap()
      .align_items(Alignment::End)
      .id("carousel-several")
      .child(item(
        Carousel::new(slides(8)).per_view(3).width(560.0).id("carousel-three"),
        "three in view · a turn moves by one",
      ));

    let by_itself = Row::new().width(FILL).id("carousel-autoplay").child(item(
      Carousel::new(slides(5)).width(336.0).autoplay(Duration::from_secs(3)),
      "every 3 s · it waits under the pointer and stops once you turn it",
    ));

    let rows = [
      (
        "new(slides) / slide / slides",
        "Element…",
        "",
        "The slides, any elements: a card, a picture, a form.",
      ),
      (
        "index / index_controlled / on_index_change",
        "Signal<usize> / usize, Fn(usize) / Fn(usize)",
        "0",
        "Which slide it is at, the first one shown. Without a signal the carousel keeps it.",
      ),
      (
        "per_view / gap",
        "usize / f32",
        "1 / 12",
        "How many slides the room shows at once, each its share; and the room between two.",
      ),
      (
        "wrap",
        "bool",
        "false",
        "Whether the first slide comes after the last, and the last before the first.",
      ),
      (
        "arrows / dots",
        "bool / bool",
        "true / true",
        "The round buttons beside the slides, and the dots under them.",
      ),
      (
        "autoplay",
        "Duration",
        "",
        "Turns by itself, from the last to the first; it waits under the pointer and with the focus from the \
         keyboard, and stops once a person turns it.",
      ),
      (
        "width",
        "Length",
        "Fill",
        "How wide it is, its buttons with it, and never wider than its room.",
      ),
      (
        "label / previous_label / next_label / slide_label",
        "&str / &str / &str / Fn(usize, usize) -> String",
        "",
        "Its words to tooling: \"Carousel\", \"Previous slide\", \"Next slide\", \"Slide 2 of 4\".",
      ),
      ("disabled", "bool", "false", "Half the opacity; it takes nothing."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces", "Carousel"],
        title: "Carousel",
        description: "Slides one after another in a room that shows one, or a few, at a time: turned by the round \
                      buttons beside them, the dots under them, the arrow keys, a swipe or a drag.",
        import: Some("use lurq::ui::Carousel;"),
        status: Some("Stable"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Carousel is a control on the right. The buttons turn it, the dots go to any slide, the \
           left and right arrows turn it from a button, and a drag of the mouse carries the slides.",
          playground,
        ),
        Section::new(
          "As the board draws it",
          "Four slides, at the second: the buttons beside them and the dots under them.",
          self.frame.demo(
            0,
            board,
            "Carousel::new(slides)",
            &["Carousel::new(slides).width(336.0).index(shown)"],
          ),
        ),
        Section::new(
          "Pictures",
          "Pictures as wide as the room, the first after the last.",
          self.frame.demo(
            1,
            photos,
            ".wrap(true)",
            &[
              "Carousel::new(photos.iter().map(|photo| Picture::from_file(&photo.path).aspect_ratio(16.0 / 9.0)))",
              "    .width(560.0)",
              "    .wrap(true)",
              "    .label(\"Photos\")",
            ],
          ),
        ),
        Section::new(
          "Several at once",
          "Three slides in view, each a third of the room: a turn moves by one, and the last three are one place.",
          self.frame.demo(
            2,
            several,
            ".per_view(3)",
            &["Carousel::new(slides).per_view(3).width(560.0)"],
          ),
        ),
        Section::new(
          "By itself",
          "It turns every few seconds, waits while the pointer is over it or the focus is in it from the keyboard, \
           and stops for good once a person turns it.",
          self.frame.demo(
            3,
            by_itself,
            ".autoplay(Duration::from_secs(3))",
            &["Carousel::new(slides).width(336.0).autoplay(Duration::from_secs(3))"],
          ),
        ),
        Section::new(
          "Props",
          "What a carousel is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
