//! The page of the rating: its playground, the board's rating bound to a signal, a rating shown and not chosen,
//! halves and none until given, its states, its sizes, another icon, and a rating in a field.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{Field, Interaction, Rating, RatingPrecision, RatingSize, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(Row::new().max_width(FILL).overflow_visible().child(shown))
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

/// The stars a signal holds, as the page says it.
fn says(stars: f64) -> String {
  format!("the signal has: {stars}")
}

pub struct RatingPage {
  frame: Frame,
  playground: Playground,
  stars: Signal<f64>,
  max: Signal<u32>,
  half: Signal<bool>,
  clearable: Signal<bool>,
  read_only: Signal<bool>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  size: Signal<RatingSize>,
  hearts: Signal<bool>,
  quality: Signal<f64>,
  review: Signal<Option<f64>>,
  call: Signal<f64>,
}

impl RatingPage {
  /// The playground: the rating as the panel's props make it, bound to the page's signal.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (max, half, clearable) = (self.max.get(), self.half.get(), self.clearable.get());
    let (read_only, invalid, disabled) = (self.read_only.get(), self.invalid.get(), self.disabled.get());
    let (size, hearts) = (self.size.get(), self.hearts.get());
    let stars = self.stars.clone();
    let plain = move |stars: Signal<f64>| {
      let mut rating = Rating::new(stars)
        .label("Rating")
        .max(max)
        .precision(if half {
          RatingPrecision::Half
        } else {
          RatingPrecision::Whole
        })
        .clearable(clearable)
        .read_only(read_only)
        .invalid(invalid)
        .disabled(disabled)
        .size(size);
      if hearts {
        rating = rating.icon(icons::HEART).empty_icon(icons::HEART);
      }
      rating
    };
    let record = self.playground.recorder();
    let forced = self.playground.forced();
    let shown = {
      let (plain, stars) = (plain.clone(), stars.clone());
      Shown::built(move |ctx| {
        plain(stars)
          .force(forced)
          .on_change(move |stars| record(format!("on_change({stars})")))
          .mount(ctx)
      })
    };

    let mut code = vec!["Rating::new(stars)".to_owned(), "    .label(\"Rating\")".to_owned()];
    for (on, call) in [
      (max != 5, format!("    .max({max})")),
      (half, "    .precision(RatingPrecision::Half)".to_owned()),
      (clearable, "    .clearable(true)".to_owned()),
      (read_only, "    .read_only(true)".to_owned()),
      (invalid, "    .invalid(true)".to_owned()),
      (disabled, "    .disabled(true)".to_owned()),
      (size != RatingSize::Md, format!("    .size(RatingSize::{size:?})")),
      (hearts, "    .icon(icons::HEART).empty_icon(icons::HEART)".to_owned()),
    ] {
      if on {
        code.push(call);
      }
    }
    code.push("    .mount(ctx)".to_owned());

    let (side, gap) = match size {
      RatingSize::Sm => (16, 2),
      RatingSize::Md => (20, 4),
      RatingSize::Lg => (24, 4),
    };
    let held = self.stars.get();
    let mut tree = vec![format!(
      "root      {}   {gap} apart · a slider to tooling",
      self.playground.measured()
    )];
    for index in 0..max {
      let lit = (held - f64::from(index)).clamp(0.0, 1.0);
      let branch = if index + 1 == max { "└─" } else { "├─" };
      let look = match lit {
        lit if lit >= 1.0 => "lit, warning".to_owned(),
        lit if lit <= 0.0 => "empty, input".to_owned(),
        lit => format!("{}% lit over empty", (lit * 100.0).round()),
      };
      tree.push(format!("{branch} star    {side} × {side} px   {look}"));
    }

    let sizes = [("Sm", RatingSize::Sm), ("Md", RatingSize::Md), ("Lg", RatingSize::Lg)];
    let maxes = [("3", 3_u32), ("5", 5), ("10", 10)];
    let (to_size, to_max) = (self.size.clone(), self.max.clone());
    let controls = vec![
      prop("max", "u32", segmented_fill(&maxes, &max, move |max| to_max.set(max))),
      prop(
        "size",
        "RatingSize",
        segmented_fill(&sizes, &size, move |size| to_size.set(size)),
      ),
      divider(),
      flag("half", &self.half),
      flag("clearable", &self.clearable),
      flag("read_only", &self.read_only),
      flag("invalid", &self.invalid),
      flag("disabled", &self.disabled),
      divider(),
      flag("hearts", &self.hearts),
    ];
    let resets = (
      self.stars.clone(),
      self.max.clone(),
      self.size.clone(),
      [
        self.half.clone(),
        self.clearable.clone(),
        self.read_only.clone(),
        self.invalid.clone(),
        self.disabled.clone(),
        self.hearts.clone(),
      ],
    );
    let make_stars = Signal::new(held);
    self.playground.card(
      ctx,
      Spec {
        title: "Rating",
        make: Arc::new(move || plain(make_stars.clone()).into()),
        shown,
        fill: false,
        forceable: true,
        facts: format!("{held} of {max}"),
        event: "on_change",
        code,
        tree,
        controls,
        changed: [
          max != 5,
          size != RatingSize::Md,
          half,
          clearable,
          read_only,
          invalid,
          disabled,
          hearts,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          resets.0.set(4.0);
          resets.1.set(5);
          resets.2.set(RatingSize::Md);
          for flag in &resets.3 {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for RatingPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      stars: ctx.signal(4.0),
      max: ctx.signal(5),
      half: ctx.signal(false),
      clearable: ctx.signal(false),
      read_only: ctx.signal(false),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      size: ctx.signal(RatingSize::Md),
      hearts: ctx.signal(false),
      quality: ctx.signal(4.0),
      review: ctx.signal(None),
      call: ctx.signal(0.0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let playground = self.play(ctx);
    let board = items().child(item(
      Rating::new(self.quality.clone()).label("Quality").mount(ctx),
      &says(self.quality.get()),
    ));
    let average = |stars: f64, reviews: u32| {
      Row::new()
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(Rating::at(stars).label("Average rating").size(RatingSize::Sm))
        .child(
          text(&format!("{stars} · {reviews} reviews"), 13.0, FontWeight::Normal).color(PaletteColor::MutedForeground),
        )
    };
    let shown = items()
      .child(item(Rating::at(3.7).label("Average rating"), "at(3.7)"))
      .child(item(average(4.5, 128), "small, with what the app says beside it"));
    let review = self.review.get();
    let halves = items().child(item(
      Rating::optional(self.review.clone())
        .label("Your review")
        .precision(RatingPrecision::Half)
        .clearable(true)
        .mount(ctx),
      &match review {
        Some(stars) => format!("the signal has: Some({stars})"),
        None => "the signal has: None".to_owned(),
      },
    ));
    let states = items()
      .child(item(
        Rating::controlled(3.0, |_| {}).force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(Rating::at(3.0), "read only"))
      .child(item(Rating::controlled(2.0, |_| {}).invalid(true), "invalid"))
      .child(item(Rating::controlled(3.0, |_| {}).disabled(true), "disabled"));
    let sizes = items()
      .child(item(Rating::at(4.0).size(RatingSize::Sm), "Sm · 16, 2 apart"))
      .child(item(Rating::at(4.0), "Md · 20, 4 apart"))
      .child(item(Rating::at(4.0).size(RatingSize::Lg), "Lg · 24, 4 apart"));
    let hearts = items().child(item(
      Rating::at(3.0).icon(icons::HEART).empty_icon(icons::HEART),
      "icon(icons::HEART)",
    ));
    let call = self.call.get();
    let field = items().child(item(
      Column::new().width(280.0).child({
        let field = Field::new("Call quality", Rating::new(self.call.clone()));
        if call == 0.0 {
          field.error("Rate the call to send it.")
        } else {
          field.description("How did the call sound?")
        }
      }),
      &says(call),
    ));
    let rows = [
      (
        "new",
        "Signal<f64>",
        "",
        "The stars the signal holds, 0 for none; a press or a key sets it. optional(Signal<Option<f64>>) holds \
         None until a rating is given.",
      ),
      (
        "controlled",
        "f64, impl Fn(f64)",
        "",
        "In place of a signal: the app holds the stars and is told what a press or a key asks for. \
         controlled_optional for Option<f64>.",
      ),
      (
        "at",
        "f64",
        "",
        "Shown and not chosen from: the part of a star too (3.7). It tells nothing and is no stop of Tab.",
      ),
      (
        "on_change",
        "impl Fn(f64)",
        "",
        "Called with what a press or a key sets, after the signal. Not for a press that changes nothing.",
      ),
      ("max", "u32", "5", "How many stars there are."),
      (
        "precision",
        "RatingPrecision",
        "Whole",
        "Half: a press of the half of a star under the pointer, and a key, choose halves.",
      ),
      (
        "clearable",
        "bool",
        "false",
        "A press of the star that is the value takes it back to none; so do 0, Backspace and Delete.",
      ),
      ("read_only", "bool", "false", "Shown and not chosen from."),
      (
        "invalid",
        "bool",
        "false",
        "The stars that are not lit in the destructive color, at half its opacity.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Dimmed; it takes no input and is no stop of Tab.",
      ),
      (
        "size",
        "RatingSize",
        "Md",
        "Sm 16 and 2 apart, Md 20 and 4 apart, Lg 24 and 4 apart.",
      ),
      (
        "icon, empty_icon",
        "impl Into<Name>",
        "star-fill",
        "The icon of a lit star and of one that is not.",
      ),
      (
        "label",
        "&str",
        "\"Rating\"",
        "What it is called to tooling. A field gives it its label.",
      ),
      (
        "value_text",
        "Fn(Option<f64>, u32) -> String",
        "\"3 of 5 stars\"",
        "What it says it holds to tooling; \"No rating\" for none.",
      ),
    ];
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Rating"],
        title: "Rating",
        description: "A number of stars, chosen by a press or by the keys, or shown and not chosen: the quality of a \
                      call, what others gave a package.",
        import: Some("use lurq::ui::{Rating, RatingPrecision, RatingSize};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Rating is a control on the right. The rating on the stage is live: the stars up to the one \
           under the pointer are lit as a press would set them, a press sets them, and the arrow keys, Home, End \
           and a digit do once it has the focus.",
          playground,
        ),
        Section::new(
          "Rating",
          "The board's: five stars, those lit in the warning color. The page holds the stars in a signal. It is \
           one stop of Tab, and to a screen reader a slider that says \"4 of 5 stars\".",
          self.frame.demo(
            0,
            board,
            "Rating::new(quality).label(\"Quality\").mount(ctx)",
            &[
              "// quality: Signal<f64>",
              "Rating::new(quality).label(\"Quality\").mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Shown, not chosen",
          "Rating::at shows a number of stars and tells nothing: an average lights the part of a star.",
          self.frame.demo(
            1,
            shown,
            "Rating::at(3.7)",
            &[
              "Rating::at(3.7).label(\"Average rating\")",
              "",
              "Row::new()",
              "    .child(Rating::at(4.5).label(\"Average rating\").size(RatingSize::Sm))",
              "    .child(text(\"4.5 · 128 reviews\"))",
            ],
          ),
        ),
        Section::new(
          "Halves, and none until given",
          "A rating bound to a Signal<Option<f64>> has none until a press. In halves, the half of a star under the \
           pointer decides; clearable, a press of the star that is the value takes it back.",
          self.frame.demo(
            2,
            halves,
            ".precision(RatingPrecision::Half).clearable(true)",
            &[
              "// review: Signal<Option<f64>>",
              "Rating::optional(review)",
              "    .label(\"Your review\")",
              "    .precision(RatingPrecision::Half)",
              "    .clearable(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "States",
          "The board draws a rating at rest. The ring of the focus is around the stars; an invalid rating has its \
           stars that are not lit in the destructive color, faded, and a disabled one is dimmed.",
          self.frame.demo(
            3,
            states,
            ".invalid(true)",
            &[
              "Rating::new(stars).force(Some(Interaction::FocusVisible))",
              "Rating::at(3.0)",
              "Rating::new(stars).invalid(true)",
              "Rating::new(stars).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "Md is the board's.",
          self.frame.demo(
            4,
            sizes,
            ".size(RatingSize::Sm)",
            &[
              "Rating::at(4.0).size(RatingSize::Sm)",
              "Rating::at(4.0)",
              "Rating::at(4.0).size(RatingSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "Another icon",
          "Any icon of the theme's set, lit and not.",
          self.frame.demo(
            5,
            hearts,
            ".icon(icons::HEART)",
            &["Rating::at(3.0).icon(icons::HEART).empty_icon(icons::HEART)"],
          ),
        ),
        Section::new(
          "In a field",
          "A field names the rating and judges it: until a rating is given the field says what is wrong.",
          self.frame.demo(
            6,
            field,
            "Field::new(\"Call quality\", Rating::new(call))",
            &[
              "// call: Signal<f64>",
              "let field = Field::new(\"Call quality\", Rating::new(call.clone()));",
              "if call.get() == 0.0 {",
              "    field.error(\"Rate the call to send it.\")",
              "} else {",
              "    field.description(\"How did the call sound?\")",
              "}",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a rating is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
