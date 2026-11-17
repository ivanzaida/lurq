//! The page of the picture: its playground, what the box shows while it loads and when it cannot show its
//! pixels, the fits and the anchor, the shapes, what lies over it, what a small one keeps, and the picture
//! with no frame.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::RadiusSize},
  components::{Column, Row, Stack},
  core::Signal,
  images::ImageData,
  layout::{Alignment, StackAlignment, layout_kind::Justify, text_style::FontWeight},
  node::Element,
  ui::{
    ActionSheet, Avatar, AvatarSize, Badge, Button, ButtonShape, ButtonSize, ButtonVariant, MenuItem, Picture,
    PictureFit, PicturePlaceholder, PictureShape, PictureSize, PictureVariant, icons,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  // No wider than the stage: what is shown gets narrower with it, and what is said under it wraps.
  Column::new()
    .max_width(FILL)
    .spacing(10.0)
    .child(Row::new().max_width(FILL).padding(2.0).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(24.0).wrap().align_items(Alignment::End)
}

/// The page's picture: a lake at dusk, 4 by 3. A sky from deep blue down to a pale green, a sun with a soft ring
/// about it, and a ridge before them. The gallery has no photographs; a picture with a sun at one side and a
/// ridge along the bottom shows what a fit keeps and what an anchor holds.
pub(super) fn lake(width: u32, height: u32) -> ImageData {
  const SKY: [(f32, [f32; 3]); 5] = [
    (0.0, [11.0, 23.0, 54.0]),
    (0.36, [30.0, 58.0, 138.0]),
    (0.6, [37.0, 99.0, 235.0]),
    (0.84, [94.0, 234.0, 212.0]),
    (1.0, [17.0, 94.0, 89.0]),
  ];
  // The ridge's line, across and down in shares of the picture.
  const RIDGE: [(f32, f32); 9] = [
    (0.0, 0.74),
    (0.13, 0.587),
    (0.24, 0.693),
    (0.42, 0.427),
    (0.59, 0.68),
    (0.71, 0.573),
    (0.855, 0.76),
    (1.0, 0.627),
    (2.0, 0.627),
  ];
  let mut rgba = Vec::with_capacity((width * height * 4) as usize);
  for y in 0..height {
    for x in 0..width {
      let (across, down) = ((x as f32 + 0.5) / width as f32, (y as f32 + 0.5) / height as f32);
      let step = SKY.windows(2).find(|pair| down <= pair[1].0).unwrap_or(&SKY[3..]);
      let along = ((down - step[0].0) / (step[1].0 - step[0].0)).clamp(0.0, 1.0);
      let mut color = [0, 1, 2].map(|channel| step[0].1[channel] + (step[1].1[channel] - step[0].1[channel]) * along);
      // The sun: a disc, and a ring about it that fades. Its distance is in shares of the picture's height.
      let from_sun = (((across - 0.68) * width as f32 / height as f32).powi(2) + (down - 0.3).powi(2)).sqrt();
      let sun = if from_sun < 0.064 {
        1.0
      } else {
        (1.0 - (from_sun - 0.064) / 0.07).clamp(0.0, 1.0) * 0.35
      };
      let light = [255.0, 251.0, 235.0];
      color = [0, 1, 2].map(|channel| color[channel] + (light[channel] - color[channel]) * sun);
      let ridge = RIDGE.windows(2).find(|pair| across <= pair[1].0).unwrap_or(&RIDGE[7..]);
      let line = ridge[0].1 + (ridge[1].1 - ridge[0].1) * ((across - ridge[0].0) / (ridge[1].0 - ridge[0].0));
      if down > line {
        let dark = [11.0, 18.0, 32.0];
        color = [0, 1, 2].map(|channel| color[channel] + (dark[channel] - color[channel]) * 0.65);
      }
      rgba.extend(color.map(|channel| channel.round() as u8));
      rgba.push(255);
    }
  }
  ImageData::from_rgba(rgba, width, height)
}

/// What the picture is called.
const LAKE: &str = "The lake at dusk, the ridge behind it";

/// What the playground's picture shows.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Source {
  Lake,
  /// Still fetched by the app: the box as it is while the pixels are on their way.
  Loading,
  /// As `Loading`, with how far the load has come.
  Progress,
  /// Bytes that are no picture: the decode fails.
  Broken,
  /// No source.
  Empty,
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Source {
  fn inspect(&self, formatter: &mut lurq::app::component::DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

const SOURCES: [(&str, Source); 5] = [
  ("The lake", Source::Lake),
  ("Loading", Source::Loading),
  ("Loading, 42%", Source::Progress),
  ("Broken", Source::Broken),
  ("None", Source::Empty),
];

const FITS: [(&str, PictureFit); 4] = [
  ("Cover", PictureFit::Cover),
  ("Contain", PictureFit::Contain),
  ("Fill", PictureFit::Fill),
  ("None", PictureFit::None),
];

const ANCHORS: [(&str, StackAlignment); 5] = [
  ("Center", StackAlignment::Center),
  ("TopCenter", StackAlignment::TopCenter),
  ("BottomCenter", StackAlignment::BottomCenter),
  ("CenterStart", StackAlignment::CenterStart),
  ("CenterEnd", StackAlignment::CenterEnd),
];

const SHAPES: [(&str, PictureShape); 2] = [("Rounded", PictureShape::Rounded), ("Circle", PictureShape::Circle)];

const SIZES: [(&str, Option<PictureSize>); 5] = [
  ("By its width", None),
  ("Lg", Some(PictureSize::Lg)),
  ("Md", Some(PictureSize::Md)),
  ("Sm", Some(PictureSize::Sm)),
  ("Xs", Some(PictureSize::Xs)),
];

/// The boxes the playground's picture is put in: a width and a height.
const BOXES: [(&str, (f32, f32)); 5] = [
  ("480 × 270", (480.0, 270.0)),
  ("320 × 180", (320.0, 180.0)),
  ("200 × 113", (200.0, 113.0)),
  ("96 × 96", (96.0, 96.0)),
  ("40 × 40", (40.0, 40.0)),
];

/// The broken picture, under a key of its own: the decoder's cache keeps that it failed.
#[cfg(not(target_arch = "wasm32"))]
fn broken() -> Picture {
  Picture::from_bytes(&b"not a picture"[..]).cache_key(0x91C7_B40C)
}

/// A browser's build decodes nothing: what failed is a box with no source.
#[cfg(target_arch = "wasm32")]
fn broken() -> Picture {
  Picture::empty()
}

/// The items of the page's menu: what a picture's More opens.
fn menu() -> [MenuItem; 4] {
  [
    MenuItem::new("Copy image").icon(icons::COPY).shortcut("Ctrl+C"),
    MenuItem::new("Copy link").icon(icons::LINK),
    MenuItem::new("Open in new window").icon(icons::EXTERNAL_LINK),
    MenuItem::new("Save as…").icon(icons::DOWNLOAD).shortcut("Ctrl+S"),
  ]
}

pub struct PicturePage {
  frame: Frame,
  playground: Playground,
  source: Signal<Source>,
  label: Signal<String>,
  title: Signal<String>,
  description: Signal<String>,
  caption: Signal<String>,
  fit: Signal<PictureFit>,
  anchor: Signal<StackAlignment>,
  shape: Signal<PictureShape>,
  size: Signal<Option<PictureSize>>,
  room: Signal<(f32, f32)>,
  bare: Signal<bool>,
  badge: Signal<bool>,
  actions: Signal<bool>,
  shown: Signal<bool>,
  selectable: Signal<bool>,
  selected: Signal<bool>,
  disabled: Signal<bool>,
  /// The page's picture, drawn once: the same pixels each render.
  lake: ImageData,
}

impl PicturePage {
  fn lake(&self) -> Picture {
    Picture::new(self.lake.clone()).label(LAKE)
  }

  /// The playground: the picture as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let source = self.source.get();
    let (label, title) = (self.label.get(), self.title.get());
    let (description, caption) = (self.description.get(), self.caption.get());
    let (fit, anchor, shape) = (self.fit.get(), self.anchor.get(), self.shape.get());
    let (size, room) = (self.size.get(), self.room.get());
    let (bare, badge, actions, shown) = (self.bare.get(), self.badge.get(), self.actions.get(), self.shown.get());
    let (selectable, selected, disabled) = (self.selectable.get(), self.selected.get(), self.disabled.get());
    let lake = self.lake.clone();
    let record = self.playground.recorder();
    let choose = self.selected.clone();

    let plain = {
      let (label, title, description, caption) = (label.clone(), title.clone(), description.clone(), caption.clone());
      move || {
        let mut picture = match source {
          Source::Lake => Picture::new(lake.clone()),
          Source::Loading => Picture::empty().loading(true),
          Source::Progress => Picture::empty().loading(true).progress(0.42),
          Source::Broken => broken(),
          Source::Empty => Picture::empty(),
        }
        .size_px(room.0, room.1)
        .fit(fit)
        .anchor(anchor)
        .shape(shape)
        .bare(bare)
        .selectable(selectable)
        .selected(selected)
        .disabled(disabled);
        if !label.is_empty() {
          picture = picture.label(&label);
        }
        if !title.is_empty() {
          picture = picture.title(&title);
        }
        if !description.is_empty() {
          picture = picture.description(&description);
        }
        if !caption.is_empty() {
          picture = picture.caption(&caption);
        }
        if let Some(size) = size {
          picture = picture.size(size);
        }
        if badge {
          picture = picture.badge(Badge::new("New"));
        }
        if shown {
          picture = picture.force_bar_shown(Some(true));
        }
        picture
      }
    };
    let live = {
      let (plain, record) = (plain.clone(), record.clone());
      move |ctx: &mut Ctx| -> Element {
        let mut picture = plain().on_click({
          let record = record.clone();
          move |_| record("on_click".to_owned())
        });
        if actions {
          let (download, retry) = (record.clone(), record.clone());
          picture = picture
            .on_download(move || download("on_download".to_owned()))
            .on_retry(move || retry("on_retry".to_owned()))
            .menu_items(menu());
        }
        if selectable {
          let (record, choose) = (record.clone(), choose.clone());
          picture = picture.on_change(move |chosen| {
            choose.set(chosen);
            record(format!("on_change({chosen})"));
          });
        }
        picture.mount(ctx)
      }
    };
    let still = {
      let plain = plain.clone();
      move || -> Element {
        let picture = plain();
        if actions {
          picture.on_download(|| {}).menu_items(menu()).into()
        } else {
          picture.into()
        }
      }
    };

    let mut code = vec![match source {
      Source::Lake => "Picture::from_file(\"photos/lake.jpg\")".to_owned(),
      Source::Loading => "Picture::empty().loading(true)".to_owned(),
      Source::Progress => "Picture::empty().loading(true).progress(0.42)".to_owned(),
      Source::Broken => "Picture::from_bytes(broken_bytes)".to_owned(),
      Source::Empty => "Picture::empty()".to_owned(),
    }];
    if !label.is_empty() {
      code.push(format!("    .label({label:?})"));
    }
    code.push(format!("    .size_px({:.1}, {:.1})", room.0, room.1));
    if fit != PictureFit::Cover {
      code.push(format!("    .fit(PictureFit::{fit:?})"));
    }
    if anchor != StackAlignment::Center {
      code.push(format!("    .anchor(StackAlignment::{anchor:?})"));
    }
    if shape != PictureShape::Rounded {
      code.push(format!("    .shape(PictureShape::{shape:?})"));
    }
    if bare {
      code.push("    .bare(true)".to_owned());
    }
    if let Some(size) = size {
      code.push(format!("    .size(PictureSize::{size:?})"));
    }
    if !title.is_empty() {
      code.push(format!("    .title({title:?})"));
    }
    if !description.is_empty() {
      code.push(format!("    .description({description:?})"));
    }
    if badge {
      code.push("    .badge(Badge::new(\"New\"))".to_owned());
    }
    if actions {
      code.push("    .on_download(|| save())".to_owned());
      code.push("    .menu_items(items)".to_owned());
    }
    if shown {
      code.push("    .force_bar_shown(Some(true))".to_owned());
    }
    if selectable {
      code.push("    .selectable(true)".to_owned());
      code.push(format!("    .selected({selected})"));
      code.push("    .on_change(|chosen| ..)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    if !caption.is_empty() {
      code.push(format!("    .caption({caption:?})"));
    }
    code.push("    .on_click(|_| open())".to_owned());

    let form = size.unwrap_or_else(|| PictureSize::for_width(room.0));
    let mut tree = vec![format!(
      "picture ({})   {}",
      if selectable { "checkbox" } else { "button" },
      self.playground.measured()
    )];
    tree.push(
      match source {
        Source::Lake => "├─ image      the pixels, placed by the fit and the anchor",
        Source::Loading => "├─ sheen      the skeleton, while the pixels are on their way",
        Source::Progress => "├─ progress   the rail of the load",
        Source::Broken => "├─ error      its glyph, its name, that it did not load, the button",
        Source::Empty => "├─ center     the glyph and what it would be called",
      }
      .to_owned(),
    );
    if badge && matches!(form, PictureSize::Lg | PictureSize::Md) {
      tree.push("├─ top        the badge".to_owned());
    }
    if selectable || selected {
      tree.push("├─ select     the mark".to_owned());
    }
    if matches!(form, PictureSize::Lg | PictureSize::Md) && (!title.is_empty() || actions) {
      tree.push("├─ controls   the title, the line and the buttons, on a gradient".to_owned());
    }
    tree.push(
      if bare {
        "└─ (no edge: bare)"
      } else {
        "└─ edge       the line around the box"
      }
      .to_owned(),
    );

    let facts = format!("{form:?} · {fit:?}");
    let (to_source, to_label, to_title) = (self.source.clone(), self.label.clone(), self.title.clone());
    let (to_description, to_caption, to_fit) = (self.description.clone(), self.caption.clone(), self.fit.clone());
    let (to_anchor, to_shape, to_size) = (self.anchor.clone(), self.shape.clone(), self.size.clone());
    let (to_room, to_bare, to_badge) = (self.room.clone(), self.bare.clone(), self.badge.clone());
    let (to_actions, to_shown, to_selectable) = (self.actions.clone(), self.shown.clone(), self.selectable.clone());
    let (to_selected, to_disabled) = (self.selected.clone(), self.disabled.clone());
    let (pick_source, pick_fit, pick_anchor) = (self.source.clone(), self.fit.clone(), self.anchor.clone());
    let (pick_shape, pick_size, pick_room) = (self.shape.clone(), self.size.clone(), self.room.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Picture",
        make: Arc::new(still),
        shown: Shown::built(live),
        fill: false,
        forceable: false,
        facts,
        event: "on_click",
        code,
        tree,
        controls: vec![
          prop(
            "source",
            "the picture",
            self
              .playground
              .select(0, &SOURCES, &source, move |which| pick_source.set(which)),
          ),
          prop("label", "&str", field(&self.label)),
          divider(),
          prop(
            "size_px",
            "f32, f32",
            self
              .playground
              .select(1, &BOXES, &room, move |which| pick_room.set(which)),
          ),
          prop(
            "fit",
            "PictureFit",
            self.playground.select(2, &FITS, &fit, move |which| pick_fit.set(which)),
          ),
          prop(
            "anchor",
            "StackAlignment",
            self
              .playground
              .select(3, &ANCHORS, &anchor, move |which| pick_anchor.set(which)),
          ),
          prop(
            "shape",
            "PictureShape",
            self
              .playground
              .select(4, &SHAPES, &shape, move |which| pick_shape.set(which)),
          ),
          prop(
            "size",
            "PictureSize",
            self
              .playground
              .select(5, &SIZES, &size, move |which| pick_size.set(which)),
          ),
          flag("bare", &self.bare),
          divider(),
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          flag("badge", &self.badge),
          flag("on_download, menu_items", &self.actions),
          flag("force_bar_shown", &self.shown),
          divider(),
          flag("selectable", &self.selectable),
          flag("selected", &self.selected),
          flag("disabled", &self.disabled),
          divider(),
          prop("caption", "&str", field(&self.caption)),
        ],
        changed: [
          source != Source::Lake,
          label != LAKE,
          title != "Lake at dusk",
          description != "Ivan Zaida · 4032 × 3024",
          !caption.is_empty(),
          fit != PictureFit::Cover,
          anchor != StackAlignment::Center,
          shape != PictureShape::Rounded,
          size.is_some(),
          room != (480.0, 270.0),
          bare,
          badge,
          !actions,
          shown,
          selectable,
          selected,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_source.set(Source::Lake);
          to_label.set(LAKE.to_owned());
          to_title.set("Lake at dusk".to_owned());
          to_description.set("Ivan Zaida · 4032 × 3024".to_owned());
          to_caption.set(String::new());
          to_fit.set(PictureFit::Cover);
          to_anchor.set(StackAlignment::Center);
          to_shape.set(PictureShape::Rounded);
          to_size.set(None);
          to_room.set((480.0, 270.0));
          to_bare.set(false);
          to_badge.set(false);
          to_actions.set(true);
          to_shown.set(false);
          to_selectable.set(false);
          to_selected.set(false);
          to_disabled.set(false);
        }),
      },
    )
  }
}

impl Component for PicturePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 6),
      source: ctx.signal(Source::Lake),
      label: ctx.signal(LAKE.to_owned()),
      title: ctx.signal("Lake at dusk".to_owned()),
      description: ctx.signal("Ivan Zaida · 4032 × 3024".to_owned()),
      caption: ctx.signal(String::new()),
      fit: ctx.signal(PictureFit::Cover),
      anchor: ctx.signal(StackAlignment::Center),
      shape: ctx.signal(PictureShape::Rounded),
      size: ctx.signal(None),
      room: ctx.signal((480.0, 270.0)),
      bare: ctx.signal(false),
      badge: ctx.signal(false),
      actions: ctx.signal(true),
      shown: ctx.signal(false),
      selectable: ctx.signal(false),
      selected: ctx.signal(false),
      disabled: ctx.signal(false),
      lake: lake(640, 480),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let stage = |picture: Picture| picture.size_px(432.0, 243.0);
    let with_bar = |picture: Picture| {
      picture
        .title("Lake at dusk")
        .description("Ivan Zaida · 4032 × 3024")
        .on_download(|| {})
        .menu_items(menu())
        .force_bar_shown(Some(true))
    };

    let large = items()
      .id("picture-large")
      .child(item(
        self
          .lake()
          .title("Lake at dusk")
          .size_px(640.0, 360.0)
          .lightbox(true)
          .id("picture-opens"),
        "picture · 640 × 360 · cover · a press opens it in a lightbox",
      ))
      .child(item(
        self
          .lake()
          .width(212.0)
          .aspect_ratio(3.0 / 2.0)
          .caption("Figure 1. The lake at dusk."),
        "figure · a caption under it",
      ));

    let states = items()
      .id("picture-states")
      .child(item(stage(Picture::empty().loading(true)), "loading · the skeleton"))
      .child(item(
        stage(
          Picture::empty()
            .loading(true)
            .placeholder(PicturePlaceholder::blur_pixels(lake(48, 27))),
        ),
        "loading · a small version, blurred",
      ))
      .child(item(
        stage(
          Picture::empty()
            .loading(true)
            .progress(0.42)
            .progress_text(|_| "42% · 1.4 of 3.4 MB".to_owned()),
        ),
        "loading · slow, the progress",
      ))
      .child(item(
        stage(broken().label(LAKE)),
        "error · the name and a way to try again",
      ))
      .child(item(
        stage(Picture::empty().label(LAKE)),
        "no source · the glyph and the name",
      ))
      .child(item(
        stage(with_bar(self.lake()).badge(Badge::new("New"))),
        "under the pointer · the bar comes in",
      ))
      .child(item(
        stage(
          with_bar(self.lake())
            .on_click(|_| {})
            .force(Some(lurq::ui::Interaction::FocusVisible)),
        ),
        "focus-visible · a ring, the bar shown",
      ));

    let thumb = |picture: Picture| picture.size_px(160.0, 100.0).size(PictureSize::Sm);
    let in_a_row = items()
      .id("picture-row")
      .child(item(thumb(self.lake()).selectable(true), "in a row · at rest"))
      .child(item(
        thumb(self.lake()).selectable(true).force_bar_shown(Some(true)),
        "under the pointer · the mark",
      ))
      .child(item(
        thumb(self.lake())
          .selectable(true)
          .force(Some(lurq::ui::Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(thumb(self.lake()).selectable(true).selected(true), "selected"))
      .child(item(thumb(self.lake()).selectable(true).disabled(true), "disabled"));

    let fit = |fit: PictureFit, anchor: StackAlignment| self.lake().size_px(130.0, 73.0).fit(fit).anchor(anchor);
    // The picture a quarter of its size, so that its own size is larger than the box and is cut by it.
    let own = Picture::new(lake(240, 180))
      .label(LAKE)
      .size_px(130.0, 73.0)
      .fit(PictureFit::None);
    let fits = items()
      .id("picture-fits")
      .child(item(
        fit(PictureFit::Cover, StackAlignment::Center),
        "cover · the default",
      ))
      .child(item(fit(PictureFit::Contain, StackAlignment::Center), "contain"))
      .child(item(fit(PictureFit::Fill, StackAlignment::Center), "fill"))
      .child(item(own, "none · its own size"))
      .child(item(fit(PictureFit::Cover, StackAlignment::TopCenter), "cover · top"))
      .child(item(
        fit(PictureFit::Cover, StackAlignment::BottomCenter),
        "cover · bottom",
      ));

    let ratio = |ratio: f32| self.lake().height(96.0).aspect_ratio(ratio);
    let ratios = items()
      .id("picture-ratios")
      .child(item(ratio(1.0), "1:1"))
      .child(item(ratio(4.0 / 3.0), "4:3"))
      .child(item(ratio(16.0 / 9.0), "16:9"))
      .child(item(ratio(3.0 / 2.0), "3:2"))
      .child(item(ratio(3.0 / 4.0), "3:4 · portrait"))
      .child(item(ratio(1.0).shape(PictureShape::Circle), "circle"));

    let over = |picture: Picture| {
      picture
        .size_px(207.0, 116.0)
        .size(PictureSize::Lg)
        .force_bar_shown(Some(true))
    };
    let over_it = items()
      .id("picture-over")
      .child(item(
        over(self.lake()).badge(Badge::new("New")),
        "badge · top start, 12 in",
      ))
      .child(item(
        over(self.lake())
          .title("Lake at dusk")
          .description("Ivan Zaida · 4032 × 3024"),
        "caption · on the lower edge",
      ))
      .child(item(
        over(self.lake()).on_download(|| {}).menu_items(menu()),
        "actions · the lower end",
      ));

    let sizes = items()
      .id("picture-sizes")
      .child(item(
        with_bar(self.lake()).size_px(200.0, 113.0),
        "compact · the rest in the menu",
      ))
      .child(item(
        Row::new()
          .max_width(FILL)
          .spacing(8.0)
          .wrap()
          .child(self.lake().size_px(96.0, 96.0))
          .child(Picture::empty().loading(true).progress(0.42).size_px(96.0, 96.0))
          .child(broken().label(LAKE).size_px(96.0, 96.0)),
        "thumbnail · 96 · a state is a glyph or a line",
      ))
      .child(item(
        Row::new()
          .spacing(8.0)
          .child(self.lake().size_px(40.0, 40.0).shape(PictureShape::Circle))
          .child(self.lake().size_px(40.0, 40.0))
          .child(
            Picture::empty()
              .loading(true)
              .size_px(40.0, 40.0)
              .shape(PictureShape::Circle),
          )
          .child(Picture::empty().label(LAKE).size_px(40.0, 40.0)),
        "avatar-sized · 40 · nothing over it",
      ));

    let own_layer = Stack::new().size(FILL, FILL).child(
      Column::new()
        .size(FILL, FILL)
        .justify(Justify::SpaceBetween)
        .child(
          Row::new()
            .width(FILL)
            .padding(12.0)
            .align_items(Alignment::Center)
            .child(
              Row::new()
                .spacing(6.0)
                .padding_horizontal(4.0)
                .padding_vertical(4.0)
                .align_items(Alignment::Center)
                .rounded(RadiusSize::Full)
                .background("#00000066")
                .child(Avatar::new("Ivan Zaida").size(AvatarSize::Sm))
                .child(
                  Row::new()
                    .padding_horizontal(4.0)
                    .child(text("@lurq.dev", 12.0, FontWeight::Medium).color("#ffffff")),
                ),
            )
            .child(Row::new().flex(1.0))
            .child(
              Button::new("Like")
                .icon(icons::HEART)
                .icon_only(true)
                .variant(ButtonVariant::Ghost)
                .shape(ButtonShape::Pill)
                .size(ButtonSize::Sm),
            ),
        )
        .child(
          Column::new()
            .width(FILL)
            .spacing(4.0)
            .padding_horizontal(16.0)
            .padding_vertical(14.0)
            .child(text("The lake at dusk", 14.0, FontWeight::SemiBold).color("#ffffff"))
            .child(
              text(
                "From the east shore, an hour after the wind dropped.",
                13.0,
                FontWeight::Normal,
              )
              .color("#ffffff"),
            ),
        ),
    );
    let on_a_phone = items()
      .id("picture-phone")
      .child(item(
        self
          .lake()
          .size_px(360.0, 240.0)
          .variant(PictureVariant::Phone)
          .title("Lake at dusk")
          .description("Ivan Zaida · 4032 × 3024")
          .badge(Badge::new("New"))
          .on_download(|| {})
          .menu_items([MenuItem::new("Copy"), MenuItem::new("Share…")])
          .lightbox(true)
          .id("picture-phone-lake"),
        "phone · 360 × 240 · a tap opens it, a long press its actions",
      ))
      .child(item(
        Column::new().width(360.0).max_width(FILL).child(
          ActionSheet::new(
            Signal::new(true),
            [
              MenuItem::new("Download"),
              MenuItem::new("Copy"),
              MenuItem::new("Share…"),
            ],
          )
          .title("Lake at dusk")
          .inline(true),
        ),
        "long press · the actions in a sheet",
      ));

    let without = items()
      .id("picture-without")
      .child(item(
        stage(self.lake()).bare(true),
        "bare · no box, no corners, no line",
      ))
      .child(item(
        stage(self.lake()).bar(false).child(own_layer),
        "your own layer over the picture",
      ));

    let rows = [
      (
        "new / from_file / from_bytes / from_resource / from_url / from_source / empty",
        "the source",
        "",
        "Pixels the app has, a file or bytes decoded off the UI thread, a resource of the app, an address on the \
         network, or nothing: a box that says it has no picture.",
      ),
      (
        "label",
        "&str",
        "",
        "What the picture is called: what it shows of itself where there are no pixels, and its name to tooling. \
         An alt text. `decorative(true)` for a picture that says nothing the words around it do not.",
      ),
      (
        "width / height",
        "Length",
        "",
        "As a browser's img: with neither it is as large as its pixels, with one the other follows their ratio, \
         with both it is a box. The width is the most it takes: in a narrower room a box of pixels keeps its \
         shape.",
      ),
      (
        "aspect_ratio",
        "f32",
        "",
        "The box's width over its height: it holds the box of a picture whose pixels are not there yet.",
      ),
      (
        "fit",
        "PictureFit",
        "Cover",
        "Cover, Contain, Fill, None (its own size).",
      ),
      (
        "anchor",
        "StackAlignment",
        "Center",
        "Where the pixels are held in the box where they are cut or do not fill it.",
      ),
      ("shape", "PictureShape", "Rounded", "Rounded, Circle."),
      (
        "bare",
        "bool",
        "false",
        "The pixels alone: no fill, no corners, no line around them.",
      ),
      (
        "size",
        "PictureSize",
        "by its width",
        "Lg from 240 wide, Md under it, Sm at 96 and under, Xs at 40 and under: what a small picture keeps of \
         what lies over it.",
      ),
      (
        "placeholder",
        "PicturePlaceholder",
        "Skeleton",
        "What the box shows while the pixels are on their way: the sheen, a small version of the picture blurred \
         (`PicturePlaceholder::blur`), or the box alone.",
      ),
      (
        "loading",
        "bool",
        "false",
        "The box as it is while the pixels are on their way, for a picture the app is still fetching.",
      ),
      (
        "progress",
        "Option<f64>",
        "None",
        "How far the load has come, of 1: a rail in place of the sheen. `progress_text` says the words under it.",
      ),
      (
        "error_description / retry_label",
        "&str",
        "",
        "The line that says a picture did not load, and its button: \"The picture didn't load.\", \"Try again\".",
      ),
      (
        "on_load / on_error",
        "Fn() / Fn(&str)",
        "",
        "Told once the pixels are there, and once the picture did not come, with why.",
      ),
      (
        "on_retry",
        "Fn()",
        "",
        "A press of the button that asks again, before the picture asks.",
      ),
      (
        "caption",
        "&str",
        "",
        "A caption under the picture, which makes it a figure.",
      ),
      (
        "title / description",
        "&str",
        "",
        "What the bar over the picture's bottom edge says. The bar shows under the pointer and with the focus.",
      ),
      (
        "badge",
        "Element",
        "",
        "What stands at the top of the picture, at its start.",
      ),
      (
        "on_download",
        "Fn()",
        "",
        "A press of the download button, which the bar has once this is given.",
      ),
      (
        "menu_item / menu_items",
        "MenuItem",
        "",
        "The menu behind More, which the bar has once it has an item.",
      ),
      (
        "action / actions",
        "Element",
        "",
        "Buttons of the app's own in the bar.",
      ),
      (
        "bar / force_bar_shown",
        "bool / Option<bool>",
        "true / None",
        "Whether the bar is drawn, and whether it shows whatever the pointer does.",
      ),
      (
        "child / children",
        "Element",
        "",
        "The app's own layer over the whole picture.",
      ),
      ("on_click", "Fn(&MouseEvent)", "", "A press of the picture as a whole."),
      (
        "selectable / selected / on_change",
        "bool / bool / Fn(bool)",
        "false",
        "A picture that can be chosen, whether it is, and what a press would make it.",
      ),
      (
        "lightbox / open / on_open_change",
        "bool / Signal<bool> / Fn(bool)",
        "false",
        "A press, and the last button of a large picture's bar, open the picture in a Lightbox over the window; \
         whether it is open, to read and to set, and what tells of it. A picture that is chosen by a press is \
         opened by the button.",
      ),
      (
        "variant",
        "PictureVariant",
        "by the density",
        "Desktop, Phone: the phone's in an app for a finger (Density::Touch), unless told. A large picture runs \
         edge to edge, its bar is always there with no buttons, and a long press brings its actions in a sheet.",
      ),
      ("disabled", "bool", "false", "Half the opacity; it takes nothing."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Picture"],
        title: "Picture",
        description: "An image in its box: it covers the box unless told to contain, fill or keep its own size, shows \
                      a skeleton while it loads and its name when it cannot be shown, and carries a badge, a bar \
                      and the app's own layer over it.",
        import: Some("use lurq::ui::Picture;"),
        status: Some("Stable"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Picture is a control on the right. Its menu opens and its buttons have their tooltips; a \
           smaller box shows what a small picture keeps.",
          playground,
        ),
        Section::new(
          "The picture, and a figure",
          "A picture in its box, which a press opens in a lightbox, and one with a caption under it.",
          self.frame.demo(
            0,
            large,
            "Picture::from_file(path)",
            &[
              "Picture::from_file(\"photos/lake.jpg\")",
              "    .label(\"The lake at dusk\")",
              "    .size_px(640.0, 360.0)",
              "    .lightbox(true)",
              "",
              "Picture::from_file(\"photos/lake.jpg\")",
              "    .label(\"The lake at dusk\")",
              "    .width(212.0)",
              "    .aspect_ratio(3.0 / 2.0)",
              "    .caption(\"Figure 1. The lake at dusk.\")",
            ],
          ),
        ),
        Section::new(
          "States",
          "While the pixels are on their way the box shows the skeleton's sheen, or how far the load has come where \
           the app knows. A picture that did not load shows its name, that it did not load and a button that asks \
           again; one with no source a glyph and its name. Under the pointer and with the focus the bar comes in.",
          self.frame.demo(
            1,
            states,
            ".loading(true)",
            &[
              "Picture::empty().loading(true)",
              "Picture::empty().loading(true).progress(0.42).progress_text(|_| \"42% · 1.4 of 3.4 MB\".into())",
              "Picture::from_bytes(broken_bytes).label(\"The lake at dusk\")",
              "Picture::empty().label(\"The lake at dusk\")",
              "picture.title(\"Lake at dusk\").description(\"Ivan Zaida\").on_download(save).menu_items(items)",
            ],
          ),
        ),
        Section::new(
          "In a row",
          "A picture that can be chosen: the ring of its mark shows under the pointer, a press chooses it, and a \
           chosen one has its mark and a line around it.",
          self.frame.demo(
            2,
            in_a_row,
            ".selectable(true).selected(chosen)",
            &["picture.selectable(true).selected(chosen).on_change(|chosen| ..)"],
          ),
        ),
        Section::new(
          "Fit and anchor",
          "A picture of 4 by 3 in a box of 16 by 9: it covers the box, is contained in it, fills it, or keeps its own \
           size. Where it is cut, the anchor says which side is kept.",
          self.frame.demo(
            3,
            fits,
            ".fit(PictureFit::Contain)",
            &[
              "picture.fit(PictureFit::Cover)",
              "picture.fit(PictureFit::Contain)",
              "picture.fit(PictureFit::Fill)",
              "picture.fit(PictureFit::None)",
              "picture.anchor(StackAlignment::TopCenter)",
              "picture.anchor(StackAlignment::BottomCenter)",
            ],
          ),
        ),
        Section::new(
          "Ratios and a circle",
          "The box has the ratio; the picture covers it.",
          self.frame.demo(
            4,
            ratios,
            ".aspect_ratio(16.0 / 9.0)",
            &[
              "picture.height(96.0).aspect_ratio(1.0)",
              "picture.height(96.0).aspect_ratio(16.0 / 9.0)",
              "picture.height(96.0).aspect_ratio(1.0).shape(PictureShape::Circle)",
            ],
          ),
        ),
        Section::new(
          "Over the picture",
          "A badge at the top, and along the bottom a bar with what the picture is and its buttons, white on a \
           gradient to black in both themes.",
          self.frame.demo(
            5,
            over_it,
            ".badge(Badge::new(\"New\"))",
            &[
              "picture.badge(Badge::new(\"New\"))",
              "picture.title(\"Lake at dusk\").description(\"Ivan Zaida · 4032 × 3024\")",
              "picture.on_download(save).menu_items(items)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "A compact picture keeps a title on one line and the menu's button; a thumbnail the picture alone, with a \
           glyph or a line for what it does; one as small as an avatar nothing over it. The form follows the width \
           unless it is said.",
          self.frame.demo(
            6,
            sizes,
            ".size(PictureSize::Sm)",
            &[
              "picture.size_px(200.0, 113.0)",
              "picture.size_px(96.0, 96.0)",
              "picture.size_px(40.0, 40.0).shape(PictureShape::Circle)",
            ],
          ),
        ),
        Section::new(
          "Without the frame",
          "The pixels alone, and a picture with the app's own layer over it in place of the bar.",
          self.frame.demo(
            7,
            without,
            ".bare(true)",
            &["picture.bare(true)", "picture.bar(false).child(your_layer)"],
          ),
        ),
        Section::new(
          "On a phone",
          "In an app for a finger (the theme at `Density::Touch`) a large picture runs edge to edge with no corners \
           and no line, and its bar is always there with no buttons: there is no pointer to bring it in. A tap \
           opens it, and a long press brings its actions, Download and the menu's items, in an action sheet.",
          self.frame.demo(
            8,
            on_a_phone,
            ".variant(PictureVariant::Phone)",
            &[
              "// In an app at Density::Touch a picture is the phone's on its own.",
              "Picture::from_file(\"photos/lake.jpg\")",
              "    .title(\"Lake at dusk\")",
              "    .on_download(save)",
              "    .menu_items(items)",
              "    .lightbox(true)",
              "    .variant(PictureVariant::Phone)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a picture is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
