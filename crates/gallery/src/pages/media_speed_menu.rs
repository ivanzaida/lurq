//! The page of the speed menu: its playground, the menu from a player's speed button, the board's menu drawn in
//! place, and the menu opening up from a bar over a picture.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::RadiusSize},
  components::{Column, Placement, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify},
  node::{Element, Gradient, GradientStop, color::Color},
  ui::{Button, ButtonSize, ButtonVariant, MEDIA_RATES, MediaSpeedMenu, icons, media_rate_text},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .overflow_visible()
    .child(Row::new().max_width(FILL).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .overflow_visible()
    .align_items(Alignment::Start)
}

/// Where the playground's menu opens beside its button.
const PLACEMENTS: [(&str, Placement); 4] = [
  ("BottomStart", Placement::BottomStart),
  ("BottomEnd", Placement::BottomEnd),
  ("TopStart", Placement::TopStart),
  ("TopEnd", Placement::TopEnd),
];

/// The speeds the playground's menu offers: the default ones, or a longer list of the app's.
const WIDE_RATES: [f32; 8] = [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0];

/// A speed as the page's code writes it.
fn written(rate: f32) -> String {
  format!("{rate:?}")
}

pub struct MediaSpeedMenuPage {
  frame: Frame,
  playground: Playground,
  rate: Signal<usize>,
  wide: Signal<bool>,
  placement: Signal<usize>,
  inline: Signal<bool>,
  heading: Signal<String>,
  label: Signal<String>,
  play_anchor: ElementRef,
  play_open: Signal<bool>,
  // The menu from a speed button, and the one in a bar over a picture.
  speed: Signal<f32>,
  bar_anchor: ElementRef,
  bar_open: Signal<bool>,
  over_anchor: ElementRef,
  over_open: Signal<bool>,
}

impl MediaSpeedMenuPage {
  /// The playground: the menu as the panel's props make it, from a button that says the speed in use.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let rates: &[f32] = if self.wide.get() { &WIDE_RATES } else { &MEDIA_RATES };
    let wide = self.wide.get();
    let rate = rates[self.rate.get().min(rates.len() - 1)];
    let (placement, inline) = (self.placement.get().min(PLACEMENTS.len() - 1), self.inline.get());
    let (heading, label) = (self.heading.get(), self.label.get());
    let rates = rates.to_vec();
    let plain = {
      let (heading, label, rates) = (heading.clone(), label.clone(), rates.clone());
      move |open: Signal<bool>, anchor: ElementRef| {
        let mut menu = MediaSpeedMenu::new(open, anchor, rate)
          .rates(&rates)
          .heading_text(&heading)
          .placement(PLACEMENTS[placement].1)
          .inline(inline);
        if !label.is_empty() {
          menu = menu.label(&label);
        }
        menu
      }
    };
    let record = self.playground.recorder();
    let (change, dismiss, open_change) = (record.clone(), record.clone(), record);
    let (chosen, offered) = (self.rate.clone(), rates.clone());
    let menu = plain(self.play_open.clone(), self.play_anchor.clone())
      .force(self.playground.forced())
      .on_change(move |to| {
        if let Some(index) = offered.iter().position(|offered| f64::from(*offered) == to) {
          chosen.set(index);
        }
        change(format!("on_change({to})"));
      })
      .on_dismiss(move || dismiss("on_dismiss".to_owned()))
      .on_open_change(move |open| open_change(format!("on_open_change({open})")));
    let shown: Element = if inline {
      menu.into()
    } else {
      let toggle = self.play_open.clone();
      Column::new()
        .child(
          Row::new().ref_element(self.play_anchor.clone()).child(
            Button::new(&media_rate_text(rate))
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .icon(icons::GAUGE)
              .on_click(move |_| toggle.update(|open| *open = !*open)),
          ),
        )
        .child(menu)
        .into()
    };
    let make = {
      let plain = plain.clone();
      move || -> Element { plain(Signal::new(true), ElementRef::new()).inline(true).into() }
    };

    let mut code = vec![
      "// open: Signal<bool>, set by a press of the speed button; speed_button: ElementRef".to_owned(),
      format!("MediaSpeedMenu::new(open, speed_button, {})", written(rate)),
    ];
    if wide {
      code.push(format!("    .rates(&[{}])", WIDE_RATES.map(written).join(", ")));
    }
    if heading != "Playback speed" {
      code.push(format!("    .heading_text({heading:?})"));
    }
    if !label.is_empty() {
      code.push(format!("    .label({label:?})"));
    }
    if placement != 0 {
      code.push(format!("    .placement(Placement::{})", PLACEMENTS[placement].0));
    }
    if inline {
      code.push("    .inline(true)".to_owned());
    }
    code.push("    .on_change(|rate| set_rate(rate))".to_owned());

    let mut tree = vec![
      format!("panel        {}   168 wide", self.playground.measured()),
      format!("├─ label      {heading:?}   11/600"),
    ];
    for (index, offered) in rates.iter().enumerate() {
      let end = if index + 1 == rates.len() { "└─" } else { "├─" };
      let checked = if *offered == rate { "  check 16" } else { "" };
      tree.push(format!(
        "{end} item       32 tall  {:?}{checked}",
        media_rate_text(*offered)
      ));
    }

    let (pick_rate, pick_placement) = (self.rate.clone(), self.placement.clone());
    let names: Vec<String> = rates.iter().map(|rate| media_rate_text(*rate)).collect();
    let options: Vec<(&str, String)> = names.iter().map(|name| (name.as_str(), name.clone())).collect();
    let picked = names.clone();
    let normal = rates.iter().position(|rate| *rate == 1.0).unwrap_or(0);
    let (to_rate, to_wide, to_placement, to_inline, to_heading, to_label) = (
      self.rate.clone(),
      self.wide.clone(),
      self.placement.clone(),
      self.inline.clone(),
      self.heading.clone(),
      self.label.clone(),
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Speed Menu",
        make: Arc::new(make),
        shown: Shown::of(shown),
        fill: false,
        forceable: true,
        facts: format!("{} in use · {}", media_rate_text(rate), PLACEMENTS[placement].0),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop(
            "rate",
            "f32",
            self
              .playground
              .select(0, &options, &media_rate_text(rate), move |name| {
                pick_rate.set(picked.iter().position(|held| *held == name).unwrap_or(normal))
              }),
          ),
          prop(
            "placement",
            "Placement",
            self.playground.select(
              1,
              &PLACEMENTS.map(|(name, _)| (name, name)),
              &PLACEMENTS[placement].0,
              move |name| pick_placement.set(PLACEMENTS.iter().position(|(held, _)| *held == name).unwrap_or(0)),
            ),
          ),
          prop("heading_text", "&str", field(&self.heading)),
          prop("label", "&str", field(&self.label)),
          divider(),
          flag("rates: 0.25× to 3×", &self.wide),
          flag("inline", &self.inline),
        ],
        changed: [
          rate != 1.0,
          wide,
          placement != 0,
          !inline,
          heading != "Playback speed",
          !label.is_empty(),
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_wide.set(false);
          to_rate.set(2);
          to_placement.set(0);
          to_inline.set(true);
          to_heading.set("Playback speed".to_owned());
          to_label.set(String::new());
        }),
      },
    )
  }
}

/// What stands in for a picture: the board's sunset, from the top down, with what is on it at its bottom end.
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
    .max_width(FILL)
    .padding(12.0)
    .rounded(RadiusSize::Lg)
    .background_gradient(Gradient::linear(180.0, stops))
    .align_items(Alignment::End)
    .justify(Justify::End)
}

impl Component for MediaSpeedMenuPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      rate: ctx.signal(2),
      wide: ctx.signal(false),
      placement: ctx.signal(0),
      inline: ctx.signal(true),
      heading: ctx.signal("Playback speed".to_owned()),
      label: ctx.signal(String::new()),
      play_anchor: ElementRef::new(),
      play_open: ctx.signal(false),
      speed: ctx.signal(1.0),
      bar_anchor: ElementRef::new(),
      bar_open: ctx.signal(false),
      over_anchor: ElementRef::new(),
      over_open: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let speed = self.speed.get();
    // A speed button and its menu: the button says the speed in use and opens the menu, which sets it.
    let speed_button = |anchor: &ElementRef, open: &Signal<bool>, placement: Placement, on_media: bool| {
      let (toggle, set) = (open.clone(), self.speed.clone());
      let mut button = Button::new(&media_rate_text(speed))
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::Sm)
        .on_click(move |_| toggle.update(|open| *open = !*open));
      if on_media {
        let white = Color::from_hex("#FFFFFF");
        button = button
          .style(lurq::ui::ButtonPart::Root, |s| s.foreground(white))
          .style(lurq::ui::ButtonPart::Label, |s| s.foreground(white));
      }
      Column::new()
        .overflow_visible()
        .child(Row::new().ref_element(anchor.clone()).child(button))
        .child(
          MediaSpeedMenu::new(open.clone(), anchor.clone(), speed)
            .placement(placement)
            .on_change(move |rate| set.set(rate as f32)),
        )
    };
    let bound = items().child(item(
      speed_button(&self.bar_anchor, &self.bar_open, Placement::BottomStart, false),
      &format!("the page's speed: {}", media_rate_text(speed)),
    ));
    let board = items()
      .child(item(
        MediaSpeedMenu::new(Signal::new(true), ElementRef::new(), 1.0).inline(true),
        "speed menu · Normal in use",
      ))
      .child(item(
        MediaSpeedMenu::new(Signal::new(true), ElementRef::new(), 1.5).inline(true),
        "1.5× in use",
      ));
    let over = items().child(item(
      picture(320.0, 180.0).overflow_visible().child(speed_button(
        &self.over_anchor,
        &self.over_open,
        Placement::TopEnd,
        true,
      )),
      "from a bar over a picture: the menu stays themed",
    ));
    let rows = [
      (
        "new",
        "Signal<bool>, impl Into<ElementRef>, f32",
        "",
        "Open while the signal is true, hung from the anchor, with the speed in use checked.",
      ),
      (
        "on_change",
        "impl Fn(f64)",
        "",
        "Called with the speed chosen, after the menu has closed, where it is another than the one in use.",
      ),
      (
        "on_dismiss",
        "impl Fn()",
        "",
        "Called when a person closes it other than by choosing: Escape, a press outside, Tab.",
      ),
      (
        "on_open_change",
        "impl Fn(bool)",
        "",
        "Called with false when a person closes it, a choice too; after on_dismiss.",
      ),
      (
        "rates",
        "&[f32]",
        "MEDIA_RATES",
        "The speeds offered: 0.5×, 0.75×, Normal, 1.25×, 1.5× and 2× unless told.",
      ),
      (
        "heading_text",
        "&str",
        "\"Playback speed\"",
        "The words over the speeds.",
      ),
      ("label", "&str", "the heading", "The menu's name to tooling."),
      (
        "placement",
        "Placement",
        "BottomStart",
        "Where it opens beside its anchor: TopEnd from a bar at the bottom of a picture.",
      ),
      (
        "offset",
        "impl Into<SpacingValue>",
        "4",
        "How far it is from its anchor.",
      ),
      (
        "collision",
        "CollisionStrategy",
        "FlipThenClamp",
        "What it does where the window has no room for it.",
      ),
      (
        "inline",
        "bool",
        "false",
        "Drawn in place, open or not: for a gallery, a test.",
      ),
      (
        "style / rules / state",
        "MediaSpeedMenuPart, Recipe, Name",
        "",
        "A part restyled, the rules of the component it is in, a state the theme's recipe names.",
      ),
      (
        "force",
        "Option<Interaction>",
        "None",
        "Draws the speed in use hovered, pressed or focused with nothing pointing at it.",
      ),
      (
        "id / key",
        "impl Into<Arc<str>>",
        "",
        "The id of the panel; what tells it apart.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Speed Menu"],
        title: "Speed Menu",
        description: "The menu a player's speed is chosen from: the speeds under a heading, the one in use checked \
                      at the end of its row.",
        import: Some("use lurq::ui::MediaSpeedMenu;"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of MediaSpeedMenu is a control on the right. A choice sets the panel's rate, as a player \
           would; the Events tab hears the speed told, and the menu closing. Choosing the speed in use only closes \
           it.",
          playground,
        ),
        Section::new(
          "From a speed button",
          "The page holds the speed. The button says it and opens the menu; Up and Down move between the speeds, \
           Enter chooses, Escape closes.",
          self.frame.demo(
            0,
            bound,
            "MediaSpeedMenu::new(open, speed_button, speed.get())",
            &[
              "// open: Signal<bool>, speed: Signal<f32>, speed_button: ElementRef",
              "Column::new()",
              "    .child(",
              "        Row::new().ref_element(speed_button.clone()).child(",
              "            Button::new(&media_rate_text(speed.get()))",
              "                .variant(ButtonVariant::Ghost)",
              "                .on_click(move |_| open.update(|open| *open = !*open)),",
              "        ),",
              "    )",
              "    .child(",
              "        MediaSpeedMenu::new(open, speed_button, speed.get())",
              "            .on_change(move |rate| speed.set(rate as f32)),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "The board's menu",
          "The menu's panel, 168 wide, with the group label and a menu item for each speed. Drawn in place.",
          self.frame.demo(
            1,
            board,
            ".inline(true)",
            &[
              "MediaSpeedMenu::new(open, anchor, 1.0).inline(true)",
              "MediaSpeedMenu::new(open, anchor, 1.5).inline(true)",
            ],
          ),
        ),
        Section::new(
          "Over a picture",
          "From the bar at the bottom of a video it opens up from the button's end. It is the system's own surface \
           and stays themed over the picture.",
          self.frame.demo(
            2,
            over,
            ".placement(Placement::TopEnd)",
            &[
              "MediaSpeedMenu::new(open, speed_button, speed.get())",
              "    .placement(Placement::TopEnd)",
              "    .on_change(move |rate| speed.set(rate as f32))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a speed menu is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
