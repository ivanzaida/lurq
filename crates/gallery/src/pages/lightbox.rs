//! The page of the lightbox: its playground, the lightbox in a page as the board draws it, one picture alone,
//! and the lightbox over the window.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  images::ImageData,
  layout::Alignment,
  node::Element,
  ui::{Button, ButtonVariant, Lightbox, Picture},
};

use super::picture::lake;
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const COUNTS: [(&str, usize); 3] = [("1", 1), ("3", 3), ("8", 8)];
const CLOSEST: [(&str, f32); 3] = [("2 ×", 2.0), ("4 ×", 4.0), ("8 ×", 8.0)];
const ROOMS: [(&str, (f32, f32)); 3] = [
  ("560 × 380", (560.0, 380.0)),
  ("420 × 290", (420.0, 290.0)),
  ("320 × 420", (320.0, 420.0)),
];

/// What the pictures are called, in turn.
const TITLES: [(&str, &str); 4] = [
  ("Lake at dusk", "Ivan Zaida · 4032 × 3024"),
  ("The ridge", "A portrait: it stands whole between two bands"),
  ("The far shore", "A wide one: it lies whole between two bands"),
  ("The sun", ""),
];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(10.0)
    .child(Row::new().padding(2.0).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

pub struct LightboxPage {
  frame: Frame,
  playground: Playground,
  count: Signal<usize>,
  room: Signal<(f32, f32)>,
  closest: Signal<f32>,
  wrap: Signal<bool>,
  filmstrip: Signal<bool>,
  counter: Signal<bool>,
  download: Signal<bool>,
  close: Signal<bool>,
  /// Whether the lightbox over the window is open.
  open: Signal<bool>,
  /// The page's pictures, drawn once: the lake as it is, standing and lying.
  shots: [ImageData; 3],
}

/// `count` pictures: the three shots in turn, each with what it is called.
pub(super) fn pictures_of(shots: &[ImageData; 3], count: usize) -> Vec<Picture> {
  (0..count)
    .map(|at| {
      let (title, description) = TITLES[at % TITLES.len()];
      let picture = Picture::new(shots[at % shots.len()].clone()).label(title).title(title);
      match description {
        "" => picture,
        description => picture.description(description),
      }
    })
    .collect()
}

impl LightboxPage {
  fn pictures(&self, count: usize) -> Vec<Picture> {
    pictures_of(&self.shots, count)
  }

  /// The playground: a lightbox in the page, as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (count, room, closest) = (self.count.get(), self.room.get(), self.closest.get());
    let (wrap, filmstrip, counter) = (self.wrap.get(), self.filmstrip.get(), self.counter.get());
    let (download, close) = (self.download.get(), self.close.get());
    let shots = self.shots.clone();
    let record = self.playground.recorder();
    let open = Signal::new(true);

    let plain = {
      let open = open.clone();
      move || {
        Lightbox::new(open.clone(), pictures_of(&shots, count))
          .inline(true)
          .width(room.0)
          .height(room.1)
          .zoom_max(closest)
          .wrap(wrap)
          .filmstrip(filmstrip)
          .counter(counter)
          .close_button(close)
      }
    };
    let live = {
      let (plain, record) = (plain.clone(), record.clone());
      move |_: &mut Ctx| -> Element {
        let (turned, closer, dismissed) = (record.clone(), record.clone(), record.clone());
        let mut lightbox = plain()
          .on_index_change(move |index| turned(format!("on_index_change({index})")))
          .on_zoom_change(move |zoom| closer(format!("on_zoom_change({zoom:.2})")))
          .on_dismiss(move || dismissed("on_dismiss".to_owned()));
        if download {
          let record = record.clone();
          lightbox = lightbox.on_download(move |index| record(format!("on_download({index})")));
        }
        lightbox.into()
      }
    };
    let still = {
      let plain = plain.clone();
      move || -> Element {
        let lightbox = plain();
        if download {
          lightbox.on_download(|_| {}).into()
        } else {
          lightbox.into()
        }
      }
    };

    let mut code = vec!["Lightbox::new(open, pictures)".to_owned()];
    if closest != 4.0 {
      code.push(format!("    .zoom_max({closest:.1})"));
    }
    if wrap {
      code.push("    .wrap(true)".to_owned());
    }
    if !filmstrip {
      code.push("    .filmstrip(false)".to_owned());
    }
    if !counter {
      code.push("    .counter(false)".to_owned());
    }
    if download {
      code.push("    .on_download(save)".to_owned());
    }
    if !close {
      code.push("    .close_button(false)".to_owned());
    }
    let mut tree = vec![
      "root".to_owned(),
      "├─ top        the counter, and the buttons".to_owned(),
    ];
    tree.push("├─ stage      the picture, its caption, the round buttons over its sides".to_owned());
    tree.push(
      if filmstrip && count > 1 {
        "└─ filmstrip  the small pictures, the one shown marked"
      } else {
        "└─ (no strip)"
      }
      .to_owned(),
    );

    let (to_count, to_room, to_closest) = (self.count.clone(), self.room.clone(), self.closest.clone());
    let (to_wrap, to_filmstrip, to_counter) = (self.wrap.clone(), self.filmstrip.clone(), self.counter.clone());
    let (to_download, to_close) = (self.download.clone(), self.close.clone());
    let (pick_count, pick_room, pick_closest) = (self.count.clone(), self.room.clone(), self.closest.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Lightbox",
        make: Arc::new(still),
        shown: Shown::built(live),
        fill: false,
        forceable: false,
        facts: format!("{count} pictures · up to {closest} ×"),
        event: "on_index_change",
        code,
        tree,
        controls: vec![
          prop(
            "pictures",
            "how many",
            self
              .playground
              .select(0, &COUNTS, &count, move |which| pick_count.set(which)),
          ),
          prop(
            "width, height",
            "in a page",
            self
              .playground
              .select(1, &ROOMS, &room, move |which| pick_room.set(which)),
          ),
          prop(
            "zoom_max",
            "f32",
            self
              .playground
              .select(2, &CLOSEST, &closest, move |which| pick_closest.set(which)),
          ),
          divider(),
          flag("wrap", &self.wrap),
          flag("filmstrip", &self.filmstrip),
          flag("counter", &self.counter),
          divider(),
          flag("on_download", &self.download),
          flag("close_button", &self.close),
        ],
        changed: [
          count != 8,
          room != (560.0, 380.0),
          closest != 4.0,
          wrap,
          !filmstrip,
          !counter,
          !download,
          !close,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_count.set(8);
          to_room.set((560.0, 380.0));
          to_closest.set(4.0);
          to_wrap.set(false);
          to_filmstrip.set(true);
          to_counter.set(true);
          to_download.set(true);
          to_close.set(true);
        }),
      },
    )
  }
}

impl Component for LightboxPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      count: ctx.signal(8),
      room: ctx.signal((560.0, 380.0)),
      closest: ctx.signal(4.0),
      wrap: ctx.signal(false),
      filmstrip: ctx.signal(true),
      counter: ctx.signal(true),
      download: ctx.signal(true),
      close: ctx.signal(true),
      open: ctx.signal(false),
      shots: [lake(640, 480), lake(360, 540), lake(900, 400)],
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let shown = Signal::new(true);
    let in_page = Row::new()
      .width(FILL)
      .spacing(24.0)
      .wrap()
      .align_items(Alignment::End)
      .id("lightbox-in-page")
      .child(item(
        Lightbox::new(shown.clone(), self.pictures(12))
          .inline(true)
          .width(420.0)
          .height(290.0)
          .on_download(|_| {}),
        "twelve pictures · 420 × 290, as the board draws it",
      ))
      .child(item(
        Lightbox::new(shown.clone(), self.pictures(1))
          .inline(true)
          .width(420.0)
          .height(290.0),
        "one picture · no counter, no strip, no round buttons",
      ));

    let open = self.open.clone();
    let over = Row::new()
      .width(FILL)
      .id("lightbox-over")
      .child(
        Button::new("Open the lightbox")
          .variant(ButtonVariant::Outline)
          .id("lightbox-open")
          .on_click(move |_| open.set(true)),
      )
      .child(
        Lightbox::new(self.open.clone(), self.pictures(8))
          .wrap(true)
          .on_download(|_| {})
          .id("lightbox-window"),
      );

    let rows = [
      (
        "new(open, pictures)",
        "Signal<bool>, Picture…",
        "",
        "Open while the signal says so, with the pictures it shows. `picture` and `pictures` add more.",
      ),
      (
        "index / index_controlled / on_index_change",
        "Signal<usize> / usize, Fn(usize) / Fn(usize)",
        "",
        "Which picture is shown, counted from 0. Without a signal the lightbox keeps it, and starts at the first.",
      ),
      (
        "zoom / on_zoom_change / zoom_max",
        "Signal<f32> / Fn(f64) / f32",
        "1 / 4",
        "How close the picture is, 1 where it fits, and the closest it is brought.",
      ),
      (
        "filmstrip / counter",
        "bool",
        "more than one",
        "The strip of small pictures and the counter: there with more than one picture unless told.",
      ),
      (
        "counter_text",
        "Fn(usize, usize) -> String",
        "3 / 12",
        "What the counter says, from which picture it is, counted from 1, and how many there are.",
      ),
      ("wrap", "bool", "false", "The first comes after the last."),
      (
        "on_download / action / actions",
        "Fn(usize) / Element",
        "",
        "Shows Download, called with the picture shown; and buttons of the app's own before it.",
      ),
      (
        "inline / width / height",
        "bool / Length",
        "false",
        "In the page where it is declared, in place of over the window, and how large it is there.",
      ),
      (
        "on_dismiss / dismiss_on_escape / dismiss_on_veil / close_button",
        "Fn() / bool",
        "true",
        "What closes it: Escape, a press around the picture, the Close button; and what is told when it is.",
      ),
      (
        "label / close_label / previous_label / next_label / zoom_in_label / zoom_out_label / fit_label / \
         download_label",
        "&str",
        "",
        "What it and its buttons are called to tooling that reads the tree.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Lightbox"],
        title: "Lightbox",
        description: "Pictures looked at one at a time, larger and closer: the picture whole in the room it has, \
                      the counter, the buttons that bring it closer, the round buttons that turn to the next, and \
                      a strip of them all.",
        import: Some("use lurq::ui::Lightbox;"),
        status: Some("Stable"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Lightbox is a control on the right. Turn the wheel over the picture to bring it closer \
           about the pointer, drag it, press + and - and 0, and the arrows.",
          playground,
        ),
        Section::new(
          "In a page",
          "The lightbox as the board draws it, and what one picture alone leaves of it.",
          self.frame.demo(
            0,
            in_page,
            ".inline(true)",
            &[
              "Lightbox::new(open, pictures).inline(true).width(420.0).height(290.0).on_download(save)",
              "",
              "Lightbox::new(open, [picture]).inline(true).width(420.0).height(290.0)",
            ],
          ),
        ),
        Section::new(
          "Over the window",
          "Open while its signal says so. Escape, Close and a press around the picture dismiss it, and the focus \
           goes back to what had it.",
          self.frame.demo(
            1,
            over,
            "Lightbox::new(open, pictures)",
            &[
              "let open = ctx.signal(false);",
              "Button::new(\"Open the lightbox\").on_click(move |_| open.set(true));",
              "Lightbox::new(open, pictures).wrap(true).on_download(save)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a lightbox is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
