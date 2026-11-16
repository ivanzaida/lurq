//! The page of the tour: its playground, the board's card, a tour with Skip, and one whose step the page holds.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Placement, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{Button, ButtonVariant, TeachingTipVariant, Tour, TourStep, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The looks the playground offers.
const VARIANTS: [(&str, TeachingTipVariant); 2] = [
  ("Plain", TeachingTipVariant::Plain),
  ("Primary", TeachingTipVariant::Primary),
];

/// The sides the playground offers.
const PLACEMENTS: [(&str, Placement); 4] = [
  ("Bottom", Placement::Bottom),
  ("BottomStart", Placement::BottomStart),
  ("Top", Placement::Top),
  ("Right", Placement::Right),
];

/// What the steps of the gallery's tours say.
const STEPS: [(&str, &str); 4] = [
  (
    "Search everything",
    "Find a file, a setting or a command from one place.",
  ),
  (
    "Switch themes here",
    "Light and dark palettes share the same roles, so nothing else changes.",
  ),
  (
    "Dock panels anywhere",
    "Drag a panel by its header and drop it on any edge of the window.",
  ),
  ("Make it yours", "Shortcuts, fonts and density are in settings."),
];

/// What a step points at: an outlined button whose root is `anchor`.
fn target(label: &str, icon: &str, anchor: &ElementRef) -> Row {
  Row::new()
    .ref_element(anchor.clone())
    .child(Button::new(label).variant(ButtonVariant::Outline).icon(icon))
}

/// The four things the tours point at, side by side, as wide as the stage: what does not fit goes on to the next
/// line.
fn targets(anchors: &[ElementRef; 4]) -> Row {
  Row::new()
    .width(FILL)
    .wrap()
    .spacing(16.0)
    .justify(Justify::Center)
    .align_items(Alignment::Center)
    .child(target("Search", icons::SEARCH, &anchors[0]))
    .child(target("Theme", icons::PALETTE, &anchors[1]))
    .child(target("Panels", icons::LAYERS, &anchors[2]))
    .child(target("Settings", icons::SETTINGS, &anchors[3]))
}

/// A tour of the four steps on `anchors`.
fn steps(tour: Tour, anchors: &[ElementRef; 4]) -> Tour {
  STEPS
    .iter()
    .zip(anchors)
    .fold(tour, |tour, ((title, description), anchor)| {
      tour.step(TourStep::new(anchor.clone(), title).description(description))
    })
}

/// A stage of a tour: what starts it, what it points at, and room under them for its card.
fn stage(start: Button, anchors: &[ElementRef; 4], says: &str, tour: Element) -> Column {
  Column::new()
    .width(FILL)
    .padding_top(16.0)
    .padding_bottom(200.0)
    .spacing(20.0)
    .align_items(Alignment::Center)
    .child(start)
    .child(targets(anchors))
    .child(quiet(says, 12.0))
    .child(tour)
}

pub struct TourPage {
  frame: Frame,
  playground: Playground,
  anchors: [ElementRef; 4],
  step: Signal<Option<usize>>,
  /// The look and the side, as indices of [`VARIANTS`] and [`PLACEMENTS`].
  variant: Signal<usize>,
  placement: Signal<usize>,
  skip: Signal<bool>,
  arrow: Signal<bool>,
  takes_focus: Signal<bool>,
  outside: Signal<bool>,
  /// The tour with Skip, and the one whose step the page holds.
  skipped: ([ElementRef; 4], Signal<Option<usize>>, Signal<String>),
  held: ([ElementRef; 4], Signal<Option<usize>>),
}

fn four() -> [ElementRef; 4] {
  [
    ElementRef::new(),
    ElementRef::new(),
    ElementRef::new(),
    ElementRef::new(),
  ]
}

impl TourPage {
  /// The playground: four targets and the tour the props make of a step on each.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (chosen_variant, chosen_placement) = (self.variant.get(), self.placement.get());
    let (variant_name, variant) = VARIANTS[chosen_variant.min(VARIANTS.len() - 1)];
    let (placement_name, placement) = PLACEMENTS[chosen_placement.min(PLACEMENTS.len() - 1)];
    let (skip, arrow, takes_focus, outside) = (
      self.skip.get(),
      self.arrow.get(),
      self.takes_focus.get(),
      self.outside.get(),
    );
    let step = self.step.get();
    let plain = move |step: Signal<Option<usize>>, anchors: &[ElementRef; 4]| {
      steps(Tour::new(step), anchors)
        .variant(variant)
        .placement(placement)
        .skip_button(skip)
        .arrow(arrow)
        .takes_focus(takes_focus)
        .dismiss_on_outside_press(outside)
    };
    let record = self.playground.recorder();
    let (changed, finished, dismissed) = (record.clone(), record.clone(), record);
    let tour = plain(self.step.clone(), &self.anchors)
      .on_change(move |to| {
        changed(match to {
          Some(step) => format!("on_change(Some({step}))"),
          None => "on_change(None)".to_owned(),
        })
      })
      .on_finish(move || finished("on_finish".to_owned()))
      .on_dismiss(move || dismissed("on_dismiss".to_owned()));
    let start = {
      let step = self.step.clone();
      Button::new(if step.get().is_some() {
        "Start again"
      } else {
        "Start the tour"
      })
      .variant(ButtonVariant::Secondary)
      .on_click(move |_| step.set(Some(0)))
    };
    let shown = stage(
      start,
      &self.anchors,
      "Right and Left step while the focus is in the card · Escape leaves",
      tour.mount(ctx),
    );

    let mut code = vec![
      "// step: Signal<Option<usize>>; search, theme, panels, settings: ElementRef".to_owned(),
      "Tour::new(step.clone())".to_owned(),
      "    .step(TourStep::new(search, \"Search everything\").description(..))".to_owned(),
      "    .step(TourStep::new(theme, \"Switch themes here\").description(..))".to_owned(),
      "    .step(TourStep::new(panels, \"Dock panels anywhere\").description(..))".to_owned(),
      "    .step(TourStep::new(settings, \"Make it yours\").description(..))".to_owned(),
    ];
    if variant != TeachingTipVariant::Plain {
      code.push(format!("    .variant(TeachingTipVariant::{variant_name})"));
    }
    if placement != Placement::Bottom {
      code.push(format!("    .placement(Placement::{placement_name})"));
    }
    if skip {
      code.push("    .skip_button(true)".to_owned());
    }
    if !arrow {
      code.push("    .arrow(false)".to_owned());
    }
    if !takes_focus {
      code.push("    .takes_focus(false)".to_owned());
    }
    if outside {
      code.push("    .dismiss_on_outside_press(true)".to_owned());
    }
    code.push("    .on_finish(|| remember_the_tour_was_taken())".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let tree = vec![
      format!(
        "tour          {}",
        match step {
          Some(step) => format!("step {} of 4", step + 1),
          None => "not on".to_owned(),
        }
      ),
      format!("└─ tip        280 wide · {variant_name} · radius lg · shadow md · padded 16   a dialog"),
      "   ├─ text    title 14 medium · description 13 muted, lines of 1.5".to_owned(),
      format!(
        "   ├─ footer  \"2 of 4\" mono 11 · {}Back · Next, 28 high",
        if skip { "Skip · " } else { "" }
      ),
      format!(
        "   └─ arrow   {}",
        if arrow {
          "18 × 9, the panel's fill and edge"
        } else {
          "none"
        }
      ),
    ];

    let to = (self.step.clone(), self.variant.clone(), self.placement.clone());
    let flags = [
      self.skip.clone(),
      self.arrow.clone(),
      self.takes_focus.clone(),
      self.outside.clone(),
    ];
    let (set_variant, set_placement) = (self.variant.clone(), self.placement.clone());
    let variants: Vec<(&str, usize)> = VARIANTS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let placements: Vec<(&str, usize)> = PLACEMENTS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Tour",
        // On its own a tour is the card of its step: the board's "2 of 4".
        make: Arc::new(move || plain(Signal::new(Some(1)), &four()).into()),
        shown: Shown::of(shown),
        fill: true,
        forceable: false,
        facts: match step {
          Some(step) => format!("step {} of 4", step + 1),
          None => "not on".to_owned(),
        },
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop(
            "variant",
            "TeachingTipVariant",
            self
              .playground
              .select(0, &variants, &chosen_variant, move |index| set_variant.set(index)),
          ),
          prop(
            "placement",
            "Placement",
            self
              .playground
              .select(1, &placements, &chosen_placement, move |index| set_placement.set(index)),
          ),
          divider(),
          flag("skip_button", &self.skip),
          flag("arrow", &self.arrow),
          flag("takes_focus", &self.takes_focus),
          flag("dismiss_on_outside_press", &self.outside),
        ],
        changed: [
          variant != TeachingTipVariant::Plain,
          placement != Placement::Bottom,
          skip,
          !arrow,
          !takes_focus,
          outside,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to.0.set(None);
          to.1.set(0);
          to.2.set(0);
          flags[0].set(false);
          flags[1].set(true);
          flags[2].set(true);
          flags[3].set(false);
        }),
      },
    )
  }
}

impl Component for TourPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      anchors: four(),
      step: ctx.signal(None),
      variant: ctx.signal(0),
      placement: ctx.signal(0),
      skip: ctx.signal(false),
      arrow: ctx.signal(true),
      takes_focus: ctx.signal(true),
      outside: ctx.signal(false),
      skipped: (four(), ctx.signal(None), ctx.signal(String::new())),
      held: (four(), ctx.signal(None)),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // The board's card: the second step of four, as a tour converted on its own draws it.
    let board = Row::new()
      .width(FILL)
      .spacing(24.0)
      .wrap()
      .overflow_visible()
      .align_items(Alignment::Start)
      .child(
        Column::new()
          .spacing(10.0)
          .max_width(FILL)
          .overflow_visible()
          .child(steps(Tour::new(Signal::new(Some(1))), &four()).arrow(false))
          .child(quiet("tour", 12.0)),
      )
      .child(
        Column::new()
          .spacing(10.0)
          .max_width(FILL)
          .overflow_visible()
          .child(
            steps(Tour::new(Signal::new(Some(3))), &four())
              .arrow(false)
              .variant(TeachingTipVariant::Primary),
          )
          .child(quiet("the last step · primary", 12.0)),
      );

    let (anchors, step, left) = &self.skipped;
    let (finished, dismissed) = (left.clone(), left.clone());
    let skipping = steps(Tour::new(step.clone()), anchors)
      .skip_button(true)
      .on_finish(move || finished.set("finished".to_owned()))
      .on_dismiss(move || dismissed.set("skipped".to_owned()));
    let start = {
      let (step, left) = (step.clone(), left.clone());
      Button::new("Start the tour")
        .variant(ButtonVariant::Secondary)
        .on_click(move |_| {
          left.set(String::new());
          step.set(Some(0))
        })
    };
    let skipped = stage(
      start,
      anchors,
      &match left.get().as_str() {
        "" => "Skip leaves as Escape does, and tells on_dismiss; Done tells on_finish".to_owned(),
        how => format!("the tour was {how}"),
      },
      skipping.mount(ctx),
    );

    let (anchors, step) = &self.held;
    let follow = step.clone();
    // The page holds the step, and lets the tour go on only: Back asks, and is not followed.
    let holding = steps(
      Tour::controlled(step.get(), move |to| {
        let now = follow.get_untracked();
        if to.is_none() || to > now {
          follow.set(to);
        }
      }),
      anchors,
    );
    let start = {
      let step = step.clone();
      Button::new("Start the tour")
        .variant(ButtonVariant::Secondary)
        .on_click(move |_| step.set(Some(0)))
    };
    let held = stage(
      start,
      anchors,
      "the page follows Next and Done, and not Back",
      holding.mount(ctx),
    );

    let rows = [
      (
        "new",
        "Signal<Option<usize>>",
        "",
        "Which step shows, counted from 0; None while the tour is not on. Its buttons and its keys set it.",
      ),
      (
        "controlled",
        "(Option<usize>, impl Fn(Option<usize>))",
        "",
        "The step as a value the app holds: the handler is told the step a person asks for, and the app gives it, \
         or does not.",
      ),
      (
        "mount",
        "(ctx)",
        "",
        "Puts it in the app; the element it returns has no size.",
      ),
      (
        "step / steps",
        "TourStep",
        "",
        "Adds a step: TourStep::new(anchor, title), with description, media, placement, id and key.",
      ),
      (
        "on_change",
        "impl Fn(Option<usize>)",
        "",
        "Told the step it is at after a person changed it, None as they end or leave it; not when the app sets the \
         signal.",
      ),
      (
        "on_finish",
        "impl Fn()",
        "",
        "Told after the last step's button ended the tour.",
      ),
      (
        "on_dismiss",
        "impl Fn()",
        "",
        "Told after Escape, Skip or a press outside left the tour before its end.",
      ),
      (
        "skip_button",
        "bool",
        "false",
        "Whether every step but the last has a quiet Skip before Back and Next. The board draws none.",
      ),
      (
        "back_label, next_label, done_label, skip_label",
        "&str",
        "Back, Next, Done, Skip",
        "What its buttons say.",
      ),
      (
        "takes_focus",
        "bool",
        "true",
        "Whether the focus goes into the card as the tour starts and back to what had it as it ends. The arrows \
         step while the focus is in the card.",
      ),
      ("dismiss_on_escape", "bool", "true", "Whether Escape leaves the tour."),
      (
        "dismiss_on_outside_press",
        "bool",
        "false",
        "Whether a press outside the card leaves the tour.",
      ),
      (
        "variant, placement, offset, collision, arrow",
        "",
        "",
        "As a teaching tip has them, for every step; a step's placement comes first.",
      ),
      (
        "state, style, rules, id, key",
        "",
        "",
        "Laid on the card that shows, by the parts of TeachingTip; a step's id and key come first.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces & overlays", "Tour"],
        title: "Tour",
        description: "The steps of a walk over an app: a card at one thing after another, each with its count and a \
                      button back and on. Every step is a teaching tip; the tour holds the focus in its card, steps \
                      on the arrow keys, and gives the focus back when it ends.",
        import: Some("use lurq::ui::{Tour, TourStep};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Start the tour and go through it: with the pointer, with Enter on the button that goes on, or with the \
           arrow keys. Each change of step is counted.",
          playground,
        ),
        Section::new(
          "Tour",
          "The board's card: a title over two lines, the count in mono at the start of the footer, Back outlined \
           and Next primary. The last step's button is Done.",
          self.frame.demo(
            0,
            board,
            "Tour::new(step).step(TourStep::new(anchor, \"Switch themes here\").description(..))",
            &[
              "// step: Signal<Option<usize>>; search, theme, panels, settings: ElementRef",
              "Tour::new(step.clone())",
              "    .step(TourStep::new(search, \"Search everything\").description(..))",
              "    .step(TourStep::new(theme, \"Switch themes here\").description(..))",
              "    .step(TourStep::new(panels, \"Dock panels anywhere\").description(..))",
              "    .step(TourStep::new(settings, \"Make it yours\").description(..))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Skip",
          "A quiet button before the others leaves the tour before its end, as Escape does: the app hears \
           on_dismiss, and on_finish only after Done.",
          self.frame.demo(
            1,
            skipped,
            ".skip_button(true).on_finish(..).on_dismiss(..)",
            &[
              "Tour::new(step.clone())",
              "    .steps(steps)",
              "    .skip_button(true)",
              "    .on_finish(move || taken.set(true))",
              "    .on_dismiss(move || remind_later())",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Controlled",
          "Given the step as a value, the tour only tells what it is asked to be at. Here the page follows it \
           forward and not back.",
          self.frame.demo(
            2,
            held,
            "Tour::controlled(step, on_change)",
            &[
              "// step: Signal<Option<usize>>, read by the component that renders the tour",
              "Tour::controlled(step.get(), move |to| {",
              "    if to.is_none() || to > step.get_untracked() {",
              "        step.set(to);",
              "    }",
              "})",
              ".steps(steps)",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a tour is given.",
          props_table(&rows, &["new", "mount"], self.frame.room()),
        ),
      ],
    )
  }
}
