//! The page of the gallery: its playground, the gallery as the board draws it, one that shows fewer pictures
//! than it has, and one whose pictures are chosen by the app's keys.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  images::ImageData,
  layout::Alignment,
  node::Element,
  ui::Gallery,
};

use super::{lightbox::pictures_of, picture::lake};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const COUNTS: [(&str, usize); 3] = [("8", 8), ("12", 12), ("20", 20)];
const COLUMNS: [(&str, usize); 3] = [("3", 3), ("4", 4), ("6", 6)];
const MOST: [(&str, Option<usize>); 3] = [("all of them", None), ("8", Some(8)), ("4", Some(4))];
const WIDTHS: [(&str, f32); 3] = [("392", 392.0), ("560", 560.0), ("280", 280.0)];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(10.0)
    .child(Row::new().padding(2.0).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

pub struct GalleryPage {
  frame: Frame,
  playground: Playground,
  count: Signal<usize>,
  columns: Signal<usize>,
  most: Signal<Option<usize>>,
  width: Signal<f32>,
  selectable: Signal<bool>,
  lightbox: Signal<bool>,
  disabled: Signal<bool>,
  /// What is chosen in the playground's gallery, and in the one that chooses by the app's keys.
  chosen: Signal<Vec<usize>>,
  named: Signal<Vec<&'static str>>,
  /// The page's pictures, drawn once: the lake as it is, standing and lying.
  shots: [ImageData; 3],
}

impl GalleryPage {
  /// The playground: a gallery as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (count, columns, most, width) = (self.count.get(), self.columns.get(), self.most.get(), self.width.get());
    let (selectable, lightbox, disabled) = (self.selectable.get(), self.lightbox.get(), self.disabled.get());
    let shots = self.shots.clone();
    let record = self.playground.recorder();
    let chosen = self.chosen.clone();

    let plain = move || {
      let mut gallery = Gallery::new(pictures_of(&shots, count))
        .columns(columns)
        .width(width)
        .selectable(selectable)
        .lightbox(lightbox)
        .disabled(disabled);
      if let Some(most) = most {
        gallery = gallery.max_shown(most);
      }
      gallery
    };
    let live = {
      let (plain, record) = (plain.clone(), record.clone());
      move |_: &mut Ctx| -> Element {
        let (opened, picked, pressed) = (record.clone(), record.clone(), record.clone());
        plain()
          .selected(chosen.clone())
          .on_open_change(move |at| opened(format!("on_open_change({at:?})")))
          .on_selection_change(move |keys| picked(format!("on_selection_change({keys:?})")))
          .on_select(move |at| pressed(format!("on_select({at})")))
          .into()
      }
    };
    let still = {
      let plain = plain.clone();
      move || -> Element { plain().into() }
    };

    let mut code = vec!["Gallery::new(pictures)".to_owned()];
    if columns != 4 {
      code.push(format!("    .columns({columns})"));
    }
    if let Some(most) = most {
      code.push(format!("    .max_shown({most})"));
    }
    if selectable {
      code.push("    .selectable(true)".to_owned());
      code.push("    .selected(chosen)".to_owned());
    }
    if !lightbox {
      code.push("    .lightbox(false)".to_owned());
      code.push("    .on_select(open_it)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    let tree = vec![
      "root".to_owned(),
      "├─ row        as many rooms as there are columns".to_owned(),
      "│  └─ cell    a picture: chosen, or under the veil of a count".to_owned(),
      if lightbox {
        "└─ lightbox   one for all of them, which takes no room"
      } else {
        "└─ (no lightbox)"
      }
      .to_owned(),
    ];

    let (to_count, to_columns, to_most) = (self.count.clone(), self.columns.clone(), self.most.clone());
    let (to_width, to_selectable, to_lightbox) = (self.width.clone(), self.selectable.clone(), self.lightbox.clone());
    let (to_disabled, to_chosen) = (self.disabled.clone(), self.chosen.clone());
    let (pick_count, pick_columns) = (self.count.clone(), self.columns.clone());
    let (pick_most, pick_width) = (self.most.clone(), self.width.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Gallery",
        make: Arc::new(still),
        shown: Shown::built(live),
        fill: false,
        forceable: false,
        facts: format!("{count} pictures · {columns} in a row"),
        event: "on_open_change",
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
            "columns",
            "usize",
            self
              .playground
              .select(1, &COLUMNS, &columns, move |which| pick_columns.set(which)),
          ),
          prop(
            "max_shown",
            "usize",
            self
              .playground
              .select(2, &MOST, &most, move |which| pick_most.set(which)),
          ),
          prop(
            "width",
            "Length",
            self
              .playground
              .select(3, &WIDTHS, &width, move |which| pick_width.set(which)),
          ),
          divider(),
          flag("selectable", &self.selectable),
          flag("lightbox", &self.lightbox),
          flag("disabled", &self.disabled),
        ],
        changed: [
          count != 12,
          columns != 4,
          most != Some(8),
          width != 392.0,
          selectable,
          !lightbox,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_count.set(12);
          to_columns.set(4);
          to_most.set(Some(8));
          to_width.set(392.0);
          to_selectable.set(false);
          to_lightbox.set(true);
          to_disabled.set(false);
          to_chosen.set(Vec::new());
        }),
      },
    )
  }
}

impl Component for GalleryPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 4),
      count: ctx.signal(12),
      columns: ctx.signal(4),
      most: ctx.signal(Some(8)),
      width: ctx.signal(392.0),
      selectable: ctx.signal(false),
      lightbox: ctx.signal(true),
      disabled: ctx.signal(false),
      chosen: ctx.signal(Vec::new()),
      named: ctx.signal(vec!["ridge"]),
      shots: [lake(640, 480), lake(360, 540), lake(900, 400)],
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = Row::new()
      .width(FILL)
      .spacing(24.0)
      .wrap()
      .align_items(Alignment::End)
      .id("gallery-board")
      .child(item(
        Gallery::new(pictures_of(&self.shots, 8)).width(392.0),
        "eight pictures · four in a row, 92 × 68, 8 apart",
      ))
      .child(item(
        Gallery::new(pictures_of(&self.shots, 12)).max_shown(8).width(392.0),
        "twelve, eight shown · the last one stands for the rest",
      ));

    let names = ["lake", "ridge", "shore", "sun"];
    let chosen = Row::new().width(FILL).id("gallery-chosen").child(item(
      Gallery::keyed(names.into_iter().zip(pictures_of(&self.shots, 4)))
        .selectable(true)
        .selected(self.named.clone())
        .width(392.0),
      "chosen by the app's keys · a press chooses, Enter opens",
    ));

    let rows = [
      (
        "new(pictures) / keyed(items)",
        "Picture… / (K, Picture)…",
        "",
        "The pictures, told apart by their place, or each with a key of the app's. `picture`, `pictures` and \
         `item` add more.",
      ),
      (
        "columns / gap / aspect_ratio",
        "usize / f32 / f32",
        "4 / 8 / 92:68",
        "How many stand in a row, the room between them, and the width of a picture's room to its height.",
      ),
      (
        "size / width",
        "PictureSize / Length",
        "Sm / Fill",
        "The form of its pictures, and how wide the gallery is.",
      ),
      (
        "max_shown / more_text",
        "usize / Fn(usize) -> String",
        "all / +5",
        "The most it shows; the last one shown then lies under a veil that says how many more there are.",
      ),
      (
        "selectable / selected / selected_controlled / on_selection_change",
        "bool / Signal<Vec<K>> / Vec<K>, Fn(&[K]) / Fn(&[K])",
        "false",
        "Pictures that can be chosen, and the keys of the ones that are. Without a signal the gallery keeps them.",
      ),
      (
        "lightbox / open / on_open_change",
        "bool / Signal<Option<usize>> / Fn(Option<usize>)",
        "true",
        "One lightbox for all its pictures, and which picture is open in it.",
      ),
      (
        "on_select",
        "Fn(usize)",
        "",
        "A press of a picture in a gallery that neither chooses nor opens.",
      ),
      (
        "label / disabled",
        "&str / bool",
        "",
        "What it is called to tooling, and a gallery that takes nothing.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Gallery"],
        title: "Gallery",
        description: "Pictures in rows of as many columns: each as wide as its share, chosen among by a press, \
                      and opened in a lightbox that holds them all.",
        import: Some("use lurq::ui::Gallery;"),
        status: Some("Stable"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Gallery is a control on the right. Press a picture to open it; with `selectable` a press \
           chooses and Enter opens. Tab reaches the gallery once, and the arrows move among its pictures.",
          playground,
        ),
        Section::new(
          "As the board draws it",
          "Two rows of four, and a gallery that shows fewer pictures than it has.",
          self.frame.demo(
            0,
            board,
            "Gallery::new(pictures)",
            &[
              "Gallery::new(pictures).width(392.0)",
              "",
              "Gallery::new(pictures).max_shown(8).width(392.0)",
            ],
          ),
        ),
        Section::new(
          "Chosen by the app's keys",
          "What is chosen is a list of the app's keys, which stays right when the pictures change.",
          self.frame.demo(
            1,
            chosen,
            "Gallery::keyed(items)",
            &[
              "let chosen = ctx.signal(vec![\"ridge\"]);",
              "Gallery::keyed(photos.iter().map(|photo| (photo.id, photo.picture())))",
              "    .selectable(true)",
              "    .selected(chosen)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a gallery is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
