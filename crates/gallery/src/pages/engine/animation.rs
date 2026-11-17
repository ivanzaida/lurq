//! The page of the engine's two timelines: a transition, which runs a property from its old value to its new one
//! when it changes, and a keyframe animation, which plays a sequence registered on the tree. Its playground plays
//! one animation as its props say; its sections are the demo's Animation page and the cards of its Dynamic
//! Keyframes page: transitions on hover, the easings side by side, every registered sequence, opacity, a
//! transition and an animation on one node, and transforms.

use std::{f32::consts::TAU, sync::Arc};

use lurq::{
  animation::{
    AnimatableProperty, Animation, AnimationDirection, AnimationFillMode, AnimationIterationCount, Easing,
    KeyframeBuilder, Keyframes, KeyframesId, StepPosition, Transition,
  },
  app::{
    Tree,
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize, Theme},
    window::Appearance,
  },
  components::{Column, Rect, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::{
    CursorIcon, Element,
    dimension::Dimension,
    transform::{Decomposed, Transform2D},
  },
  ui::Switch,
};

use super::kit::{controls, number, sample, samples, tint, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, mono, props_table},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

// The keyframes the page plays, registered on the tree by `register_keyframes`. A sequence that runs a color has one
// for the light palette and one for the dark: a keyframe holds a color, not a role of the theme.
const PULSE: KeyframesId = KeyframesId::new(0x0D00);
const COLOR_CYCLE: [KeyframesId; 2] = [KeyframesId::new(0x0D01), KeyframesId::new(0x0D02)];
const SLIDE_BOUNCE: KeyframesId = KeyframesId::new(0x0D03);
const GROW_SHRINK: KeyframesId = KeyframesId::new(0x0D04);
const SPIN: KeyframesId = KeyframesId::new(0x0D05);
const ROCK: KeyframesId = KeyframesId::new(0x0D06);
const SPIN_COLOR: [KeyframesId; 2] = [KeyframesId::new(0x0D07), KeyframesId::new(0x0D08)];
/// The playground's: one sequence for each property it runs, from its value at rest to another.
const PLAY_FADE: KeyframesId = KeyframesId::new(0x0D10);
const PLAY_SLIDE: KeyframesId = KeyframesId::new(0x0D11);
const PLAY_GROW: KeyframesId = KeyframesId::new(0x0D12);
const PLAY_TURN: KeyframesId = KeyframesId::new(0x0D13);
const PLAY_SCALE: KeyframesId = KeyframesId::new(0x0D14);
const PLAY_COLOR: [KeyframesId; 2] = [KeyframesId::new(0x0D15), KeyframesId::new(0x0D16)];

/// How far the slides go, and how wide what grows gets: they stay in a well of 200.
const SLIDE: f32 = 120.0;
const PLAY_TILE: f32 = 56.0;
const PLAY_WIDE: f32 = 176.0;

/// The keyframes of the page, told to a tree once: `engine::prepare` does.
pub fn register_keyframes(tree: &mut Tree) {
  let opacity = |value: f32| {
    move |frame: &mut KeyframeBuilder| {
      frame.set(AnimatableProperty::Opacity, value);
    }
  };
  let turn = |radians: f32| {
    move |frame: &mut KeyframeBuilder| {
      frame.set(AnimatableProperty::Transform, Decomposed::IDENTITY.with_rotate(radians));
    }
  };
  let rock = |degrees: f32| {
    move |frame: &mut KeyframeBuilder| {
      frame.set(
        AnimatableProperty::Transform,
        Decomposed::IDENTITY.with_rotate_deg(degrees),
      );
    }
  };
  let size = |width: f32, height: f32| {
    move |frame: &mut KeyframeBuilder| {
      frame.set(AnimatableProperty::Width, width);
      frame.set(AnimatableProperty::Height, height);
    }
  };
  tree.register_keyframes(
    Keyframes::new(PULSE)
      .frame(0.0, opacity(1.0))
      .frame(0.5, opacity(0.3))
      .frame(1.0, opacity(1.0)),
  );
  tree.register_keyframes(
    Keyframes::new(SLIDE_BOUNCE)
      .frame(0.0, |frame| {
        frame.set(AnimatableProperty::OffsetX, 0.0);
      })
      .frame(0.5, |frame| {
        frame.set(AnimatableProperty::OffsetX, SLIDE).easing(Easing::EASE_OUT);
      })
      .frame(1.0, |frame| {
        frame.set(AnimatableProperty::OffsetX, 0.0).easing(Easing::EASE_IN);
      }),
  );
  tree.register_keyframes(
    Keyframes::new(GROW_SHRINK)
      .frame(0.0, size(60.0, 40.0))
      .frame(0.5, size(140.0, 60.0))
      .frame(1.0, size(60.0, 40.0)),
  );
  tree.register_keyframes(Keyframes::new(SPIN).frame(0.0, turn(0.0)).frame(1.0, turn(TAU)));
  tree.register_keyframes(
    Keyframes::new(ROCK)
      .frame(0.0, rock(-15.0))
      .frame(0.5, rock(15.0))
      .frame(1.0, rock(-15.0)),
  );

  tree.register_keyframes(
    Keyframes::new(PLAY_FADE)
      .frame(0.0, opacity(1.0))
      .frame(1.0, opacity(0.15)),
  );
  tree.register_keyframes(
    Keyframes::new(PLAY_SLIDE)
      .frame(0.0, |frame| {
        frame.set(AnimatableProperty::OffsetX, 0.0);
      })
      .frame(1.0, |frame| {
        frame.set(AnimatableProperty::OffsetX, SLIDE);
      }),
  );
  tree.register_keyframes(
    Keyframes::new(PLAY_GROW)
      .frame(0.0, |frame| {
        frame.set(AnimatableProperty::Width, PLAY_TILE);
      })
      .frame(1.0, |frame| {
        frame.set(AnimatableProperty::Width, PLAY_WIDE);
      }),
  );
  tree.register_keyframes(
    Keyframes::new(PLAY_TURN)
      .frame(0.0, turn(0.0))
      .frame(1.0, turn(TAU / 2.0)),
  );
  tree.register_keyframes(
    Keyframes::new(PLAY_SCALE)
      .frame(0.0, |frame| {
        frame.set(AnimatableProperty::Transform, Decomposed::IDENTITY.with_scale(1.0, 1.0));
      })
      .frame(1.0, |frame| {
        frame.set(AnimatableProperty::Transform, Decomposed::IDENTITY.with_scale(1.6, 1.6));
      }),
  );

  // The colors are the chart colors of the theme the gallery is drawn in, in each of its palettes.
  let theme = Theme::new();
  for (index, appearance) in [Appearance::Light, Appearance::Dark].into_iter().enumerate() {
    let chart = |role: PaletteColor| theme.palette_color_for(appearance, role);
    let (one, two, three, four) = (
      chart(PaletteColor::Chart1),
      chart(PaletteColor::Chart2),
      chart(PaletteColor::Chart3),
      chart(PaletteColor::Chart4),
    );
    let fill = |color| {
      move |frame: &mut KeyframeBuilder| {
        frame.set(AnimatableProperty::BackgroundColor, color);
      }
    };
    let turning = |radians: f32, color| {
      move |frame: &mut KeyframeBuilder| {
        frame.set(AnimatableProperty::Transform, Decomposed::IDENTITY.with_rotate(radians));
        frame.set(AnimatableProperty::BackgroundColor, color);
      }
    };
    tree.register_keyframes(
      Keyframes::new(COLOR_CYCLE[index])
        .frame(0.0, fill(one))
        .frame(0.33, fill(two))
        .frame(0.66, fill(three))
        .frame(1.0, fill(one)),
    );
    tree.register_keyframes(
      Keyframes::new(SPIN_COLOR[index])
        .frame(0.0, turning(0.0, one))
        .frame(0.33, turning(TAU / 3.0, two))
        .frame(0.66, turning(TAU * 2.0 / 3.0, three))
        .frame(1.0, turning(TAU, one)),
    );
    tree.register_keyframes(
      Keyframes::new(PLAY_COLOR[index])
        .frame(0.0, fill(one))
        .frame(1.0, fill(four)),
    );
  }
}

/// Which of a pair of keyframes that run a color is for the theme `ctx` draws in: the light palette's or the dark's.
fn palette(ctx: &Ctx) -> usize {
  usize::from(ctx.theme().appearance() == Appearance::Dark)
}

/// What the playground runs: the keyframes, the property, and its value at rest and at the end, as the code says
/// them.
struct Property {
  keyframes: &'static str,
  property: &'static str,
  from: &'static str,
  to: &'static str,
}

const PROPERTIES: [Property; 6] = [
  Property {
    keyframes: "FADE",
    property: "Opacity",
    from: "1.0",
    to: "0.15",
  },
  Property {
    keyframes: "SLIDE",
    property: "OffsetX",
    from: "0.0",
    to: "120.0",
  },
  Property {
    keyframes: "GROW",
    property: "Width",
    from: "56.0",
    to: "176.0",
  },
  Property {
    keyframes: "TURN",
    property: "Transform",
    from: "Decomposed::IDENTITY",
    to: "Decomposed::IDENTITY.with_rotate(PI)",
  },
  Property {
    keyframes: "GROW",
    property: "Transform",
    from: "Decomposed::IDENTITY",
    to: "Decomposed::IDENTITY.with_scale(1.6, 1.6)",
  },
  Property {
    keyframes: "TINT",
    property: "BackgroundColor",
    from: "chart_1",
    to: "chart_4",
  },
];
const PROPERTY_CHOICES: [(&str, usize); 6] = [
  ("Opacity", 0),
  ("OffsetX", 1),
  ("Width", 2),
  ("Transform: turn", 3),
  ("Transform: scale", 4),
  ("BackgroundColor", 5),
];
const DURATIONS: [(&str, i32); 4] = [("300 ms", 300), ("800 ms", 800), ("1600 ms", 1600), ("3000 ms", 3000)];
const EASINGS: [(&str, Easing); 7] = [
  ("Easing::Linear", Easing::Linear),
  ("Easing::EASE", Easing::EASE),
  ("Easing::EASE_IN", Easing::EASE_IN),
  ("Easing::EASE_OUT", Easing::EASE_OUT),
  ("Easing::EASE_IN_OUT", Easing::EASE_IN_OUT),
  (
    "Easing::Steps { count: 6, position: StepPosition::JumpEnd }",
    Easing::Steps {
      count: 6,
      position: StepPosition::JumpEnd,
    },
  ),
  ("Easing::Spring { velocity: 0.0 }", Easing::Spring { velocity: 0.0 }),
];
/// The choices of the playground's select of an easing: an index into [`EASINGS`], which a signal keeps.
const EASING_CHOICES: [(&str, usize); 7] = [
  ("Linear", 0),
  ("EASE", 1),
  ("EASE_IN", 2),
  ("EASE_OUT", 3),
  ("EASE_IN_OUT", 4),
  ("Steps (6)", 5),
  ("Spring", 6),
];
const DELAYS: [(&str, i32); 3] = [("0", 0), ("300 ms", 300), ("1000 ms", 1000)];
/// How many times it plays; 0 is without end.
const ITERATIONS: [(&str, i32); 3] = [("1", 1), ("3", 3), ("Infinite", 0)];
const DIRECTIONS: [(&str, AnimationDirection); 4] = [
  ("Normal", AnimationDirection::Normal),
  ("Reverse", AnimationDirection::Reverse),
  ("Alternate", AnimationDirection::Alternate),
  ("AlternateReverse", AnimationDirection::AlternateReverse),
];
const DIRECTION_CHOICES: [(&str, usize); 4] =
  [("Normal", 0), ("Reverse", 1), ("Alternate", 2), ("AlternateReverse", 3)];

/// The playground's props, as they are now.
#[derive(Clone, Copy)]
struct Played {
  property: usize,
  duration: i32,
  easing: usize,
  delay: i32,
  iterations: i32,
  direction: usize,
  forwards: bool,
  /// How many times the tile was pressed: each press plays it again.
  plays: i32,
}

impl Played {
  fn animation(&self, palette: usize) -> Animation {
    let keyframes = match self.property {
      0 => PLAY_FADE,
      1 => PLAY_SLIDE,
      2 => PLAY_GROW,
      3 => PLAY_TURN,
      4 => PLAY_SCALE,
      _ => PLAY_COLOR[palette],
    };
    let mut animation = Animation::new(keyframes)
      .duration_ms(self.duration.max(1) as u64)
      .delay_ms(self.delay.max(0) as u64)
      .easing(EASINGS[self.easing.min(EASINGS.len() - 1)].1)
      .direction(DIRECTIONS[self.direction.min(DIRECTIONS.len() - 1)].1);
    animation = if self.iterations == 0 {
      animation.infinite()
    } else {
      animation.iteration_count(AnimationIterationCount::Count(self.iterations as f64))
    };
    if self.forwards && self.iterations != 0 {
      animation = animation.fill_mode(AnimationFillMode::Forwards);
    }
    animation
  }

  /// The tile that plays it. Its key changes with every prop and every press, so that each is a node of its own
  /// and plays from its start: a run is the node's, and goes on as it was when only its props change.
  fn tile(&self, palette: usize) -> Rect {
    let mut tile = Rect::new(PLAY_TILE, PLAY_TILE)
      .key(format!(
        "play-{}-{}-{}-{}-{}-{}-{}-{}",
        self.property,
        self.duration,
        self.easing,
        self.delay,
        self.iterations,
        self.direction,
        self.forwards,
        self.plays
      ))
      .rounded(RadiusSize::Md)
      .background(PaletteColor::Chart1)
      .cursor(CursorIcon::Pointer)
      .animation(self.animation(palette));
    if self.property == 1 {
      tile = tile.relative(0.0, 0.0);
    }
    tile
  }

  /// The well the tile plays in: at its start for what moves it or makes it wider, in its middle for the rest.
  fn scene(&self, tile: impl Into<Element>) -> Column {
    let along = self.property == 1 || self.property == 2;
    well().width(260.0).max_width(FILL).height(140.0).child(
      Row::new()
        .size(FILL, FILL)
        .padding_horizontal(16.0)
        .align_items(Alignment::Center)
        .justify(if along { Justify::Start } else { Justify::Center })
        .overflow_visible()
        .child(tile),
    )
  }

  fn code(&self) -> Vec<String> {
    let property = &PROPERTIES[self.property.min(PROPERTIES.len() - 1)];
    let mut code = vec![
      "// Once, on the tree:".to_owned(),
      "tree.register_keyframes(".to_owned(),
      format!("    Keyframes::new({})", property.keyframes),
      format!(
        "        .frame(0.0, |f| {{ f.set(AnimatableProperty::{}, {}); }})",
        property.property, property.from
      ),
      format!(
        "        .frame(1.0, |f| {{ f.set(AnimatableProperty::{}, {}); }}),",
        property.property, property.to
      ),
      ");".to_owned(),
      String::new(),
      format!("Rect::new({PLAY_TILE}, {PLAY_TILE})"),
    ];
    if self.property == 1 {
      code.push("    .relative(0.0, 0.0)".to_owned());
    }
    code.push("    .animation(".to_owned());
    code.push(format!("        Animation::new({})", property.keyframes));
    code.push(format!("            .duration_ms({})", self.duration));
    if self.delay != 0 {
      code.push(format!("            .delay_ms({})", self.delay));
    }
    code.push(format!(
      "            .easing({})",
      EASINGS[self.easing.min(EASINGS.len() - 1)].0
    ));
    match self.iterations {
      0 => code.push("            .infinite()".to_owned()),
      1 => {}
      count => code.push(format!(
        "            .iteration_count(AnimationIterationCount::Count({count}.0))"
      )),
    }
    if self.direction != 0 {
      code.push(format!(
        "            .direction(AnimationDirection::{})",
        DIRECTIONS[self.direction.min(DIRECTIONS.len() - 1)].0
      ));
    }
    if self.forwards && self.iterations != 0 {
      code.push("            .fill_mode(AnimationFillMode::Forwards)".to_owned());
    }
    code.push("    )".to_owned());
    code
  }
}

pub struct AnimationPage {
  frame: Frame,
  playground: Playground,
  /// An index into [`PROPERTIES`].
  property: Signal<usize>,
  duration: Signal<i32>,
  /// An index into [`EASINGS`].
  easing: Signal<usize>,
  delay: Signal<i32>,
  iterations: Signal<i32>,
  /// An index into [`DIRECTIONS`].
  direction: Signal<usize>,
  forwards: Signal<bool>,
  plays: Signal<i32>,
  /// Whether the bars of the easings are at their end, all at once.
  eased: Signal<bool>,
  /// How long the bars of the easings take, in milliseconds.
  ease_duration: Signal<i32>,
}

impl AnimationPage {
  fn played(&self) -> Played {
    Played {
      property: self.property.get(),
      duration: self.duration.get(),
      easing: self.easing.get(),
      delay: self.delay.get(),
      iterations: self.iterations.get(),
      direction: self.direction.get(),
      forwards: self.forwards.get(),
      plays: self.plays.get(),
    }
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let played = self.played();
    let property = &PROPERTIES[played.property.min(PROPERTIES.len() - 1)];
    let (record, plays) = (self.playground.recorder(), self.plays.clone());
    let press = move |_: lurq::app::events::MouseEvent| {
      plays.update(|plays| *plays += 1);
      record("on_click: play again".to_owned());
    };
    let make = move || -> Element {
      let palette = usize::from(lurq::ui::current_theme().appearance() == Appearance::Dark);
      played.scene(played.tile(palette)).into()
    };
    let shown = Shown::built(move |ctx| {
      let tile = played.tile(palette(ctx)).on_click(press);
      played.scene(self.playground.measure(tile)).into()
    })
    .measured();

    let tree = vec![
      "well   260 × 140".to_owned(),
      format!(
        "└─ rect   {}   animation {}",
        self.playground.measured(),
        property.keyframes
      ),
    ];
    let (set_property, set_duration, set_easing, set_delay, set_iterations, set_direction) = (
      self.property.clone(),
      self.duration.clone(),
      self.easing.clone(),
      self.delay.clone(),
      self.iterations.clone(),
      self.direction.clone(),
    );
    let reset = {
      let (property, duration, easing, delay, iterations, direction, forwards) = (
        self.property.clone(),
        self.duration.clone(),
        self.easing.clone(),
        self.delay.clone(),
        self.iterations.clone(),
        self.direction.clone(),
        self.forwards.clone(),
      );
      Arc::new(move || {
        property.set(1);
        duration.set(1600);
        easing.set(4);
        delay.set(0);
        iterations.set(0);
        direction.set(2);
        forwards.set(false);
      })
    };
    let mut controls = vec![
      prop(
        "keyframes",
        "AnimatableProperty",
        self
          .playground
          .select(0, &PROPERTY_CHOICES, &played.property, move |value| {
            set_property.set(value)
          }),
      ),
      prop(
        "duration_ms",
        "u64",
        self
          .playground
          .select(1, &DURATIONS, &played.duration, move |value| set_duration.set(value)),
      ),
      prop(
        "easing",
        "Easing",
        self
          .playground
          .select(2, &EASING_CHOICES, &played.easing, move |value| set_easing.set(value)),
      ),
      prop(
        "delay_ms",
        "u64",
        self
          .playground
          .select(3, &DELAYS, &played.delay, move |value| set_delay.set(value)),
      ),
      prop(
        "iteration_count",
        "AnimationIterationCount",
        self
          .playground
          .select(4, &ITERATIONS, &played.iterations, move |value| {
            set_iterations.set(value)
          }),
      ),
      prop(
        "direction",
        "AnimationDirection",
        self
          .playground
          .select(5, &DIRECTION_CHOICES, &played.direction, move |value| {
            set_direction.set(value)
          }),
      ),
    ];
    // What a run holds once it ends: a run without end never does.
    if played.iterations != 0 {
      controls.push(divider());
      controls.push(flag("fill_mode(Forwards)", &self.forwards));
    }
    self.playground.card(
      ctx,
      Spec {
        title: "Animation",
        make: Arc::new(make),
        shown,
        fill: true,
        forceable: false,
        facts: format!("{} {} → {}", property.property, property.from, property.to),
        event: "on_click",
        code: played.code(),
        tree,
        controls,
        changed: [
          played.property != 1,
          played.duration != 1600,
          played.easing != 4,
          played.delay != 0,
          played.iterations != 0,
          played.direction != 2,
          played.forwards && played.iterations != 0,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }

  /// The bars of the easings: each grows on hover, and all of them at once with the switch, in as long as the
  /// slider says.
  fn easings(&self) -> Column {
    let eased = self.eased.get();
    let duration = self.ease_duration.get().max(1) as u64;
    let set = self.eased.clone();
    let mut rows = Column::new().width(FILL).spacing(12.0).child(
      controls()
        .child(number("duration, ms", &self.ease_duration, 200, 2000))
        .child(Switch::new("At the end").on(eased).on_change(move |on| set.set(on))),
    );
    for (name, easing) in EASINGS {
      let short = name.trim_start_matches("Easing::").split(' ').next().unwrap_or(name);
      let width = if eased { 100.0 } else { 30.0 };
      rows = rows.child(
        Row::new()
          .width(FILL)
          .height(32.0)
          .spacing(12.0)
          .align_items(Alignment::Center)
          .child(mono(short, 12.0).width(96.0).flex_shrink(0.0).nowrap())
          .child(
            Row::new().flex(1.0).min_width(0.0).height(FILL).child(
              Rect::new(Dimension::Pct(width), FILL)
                .rounded(RadiusSize::Sm)
                .background(if eased {
                  PaletteColor::Chart2
                } else {
                  PaletteColor::Chart1
                })
                .transition(Transition::all().duration_ms(duration).easing(easing))
                .hovered(|style| style.background(PaletteColor::Chart2).width(Dimension::Pct(100.0)))
                .cursor(CursorIcon::Pointer),
            ),
          ),
      );
    }
    rows
  }
}

/// A tile of the `index`th tint in a slot of its own size or larger, so that one that grows on hover moves
/// nothing beside it.
fn slot(width: f32, height: f32, shown: impl Into<Element>) -> Row {
  Row::new()
    .size(width, height)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .overflow_visible()
    .child(shown)
}

fn hover_box(index: usize) -> Rect {
  Rect::new(80.0, 60.0)
    .background(tint(index))
    .rounded(RadiusSize::Md)
    .border_inside(2.0, PaletteColor::Border)
    .cursor(CursorIcon::Pointer)
}

/// The six boxes that change on hover, each with its transition.
fn hover_transitions() -> Row {
  let boxes: [(Rect, &str); 6] = [
    (
      hover_box(0)
        .transition(Transition::background_color().duration_ms(400))
        .hovered(|style| style.background(PaletteColor::Destructive)),
      "Color",
    ),
    (
      hover_box(1)
        .transition(Transition::all().duration_ms(300))
        .hovered(|style| style.border_inside(4.0, PaletteColor::Success)),
      "Border",
    ),
    (
      hover_box(2)
        .transition(Transition::all().duration_ms(500).easing(Easing::EASE_OUT))
        .hovered(|style| style.rounded(30.0)),
      "Radius",
    ),
    (
      hover_box(3)
        .transition(Transition::opacity().duration_ms(600).easing(Easing::EASE_IN_OUT))
        .hovered(|style| style.opacity(0.3)),
      "Opacity",
    ),
    (
      hover_box(4)
        .transition(Transition::all().duration_ms(400).easing(Easing::EASE_IN_OUT))
        .hovered(|style| style.size(130.0, 60.0)),
      "Width",
    ),
    (
      hover_box(0)
        .rounded(4.0)
        .transition(
          Transition::background_color()
            .duration_ms(400)
            .easing(Easing::EASE_IN_OUT),
        )
        .transition(Transition::all().duration_ms(500).easing(Easing::EASE_OUT))
        .hovered(|style| {
          style
            .background(PaletteColor::Destructive)
            .rounded(28.0)
            .border_inside(3.0, PaletteColor::Success)
            .size(130.0, 70.0)
        }),
      "All at once",
    ),
  ];
  let mut row = samples();
  for (shown, caption) in boxes {
    row = row.child(sample(slot(130.0, 72.0, shown), caption));
  }
  row
}

/// A tile in a well of a fixed size, with the sequence it plays under it.
fn keyframe_card(shown: impl Into<Element>, caption: &str) -> Column {
  sample(
    well()
      .width(200.0)
      .max_width(FILL)
      .height(110.0)
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .child(shown),
    caption,
  )
}

fn keyframe_tile(size: f32, index: usize) -> Rect {
  Rect::new(size, size).background(tint(index)).rounded(RadiusSize::Md)
}

/// Every sequence the page registers, each playing without end.
fn keyframes(palette: usize) -> Row {
  let forever =
    |keyframes: KeyframesId, duration: u64| Animation::new(keyframes).duration_ms(duration).linear().infinite();
  samples()
    .child(keyframe_card(
      keyframe_tile(64.0, 0).animation(forever(PULSE, 2000)),
      "PULSE: opacity",
    ))
    .child(keyframe_card(
      keyframe_tile(64.0, 0)
        .rounded(RadiusSize::Full)
        .animation(forever(COLOR_CYCLE[palette], 3000)),
      "COLOR_CYCLE: background",
    ))
    .child(keyframe_card(
      Row::new()
        .width(FILL)
        .padding_horizontal(16.0)
        .align_items(Alignment::Center)
        .overflow_visible()
        .child(
          keyframe_tile(48.0, 2).relative(0.0, 0.0).animation(
            Animation::new(SLIDE_BOUNCE)
              .duration_ms(2500)
              .easing(Easing::EASE_IN_OUT)
              .infinite(),
          ),
        ),
      "SLIDE_BOUNCE: offset x",
    ))
    .child(keyframe_card(
      Rect::new(60.0, 40.0)
        .background(tint(1))
        .rounded(RadiusSize::Md)
        .animation(forever(GROW_SHRINK, 2000)),
      "GROW_SHRINK: width, height",
    ))
    .child(keyframe_card(
      keyframe_tile(56.0, 2).animation(forever(SPIN, 2000)),
      "SPIN: transform",
    ))
    .child(keyframe_card(
      keyframe_tile(56.0, 3).animation(forever(ROCK, 1000)),
      "ROCK: transform",
    ))
    .child(keyframe_card(
      keyframe_tile(56.0, 0)
        .rounded(RadiusSize::Full)
        .animation(forever(SPIN_COLOR[palette], 3000)),
      "SPIN_COLOR: transform, background",
    ))
}

/// The same tile at five opacities.
fn opacities() -> Row {
  let mut row = samples();
  for (caption, value) in [("1.0", 1.0), ("0.75", 0.75), ("0.5", 0.5), ("0.25", 0.25), ("0.1", 0.1)] {
    row = row.child(sample(
      keyframe_tile(60.0, 0).opacity(value),
      &format!("opacity({caption})"),
    ));
  }
  row
}

/// A keyframe animation and a transition on the same node: the one plays, the other answers the pointer.
fn combined(palette: usize) -> Row {
  let forever =
    |keyframes: KeyframesId, duration: u64| Animation::new(keyframes).duration_ms(duration).linear().infinite();
  let base = |index: usize| {
    Rect::new(100.0, 60.0)
      .background(tint(index))
      .rounded(RadiusSize::Md)
      .cursor(CursorIcon::Pointer)
  };
  samples()
    .child(sample(
      slot(
        130.0,
        80.0,
        base(0)
          .animation(forever(PULSE, 3000))
          .transition(Transition::background_color().duration_ms(300))
          .hovered(|style| style.background(PaletteColor::Destructive)),
      ),
      "pulse, and a color on hover",
    ))
    .child(sample(
      slot(
        130.0,
        80.0,
        base(1)
          .animation(forever(COLOR_CYCLE[palette], 4000))
          .transition(Transition::all().duration_ms(400).easing(Easing::EASE_OUT))
          .hovered(|style| style.rounded(30.0)),
      ),
      "color cycle, and a radius on hover",
    ))
    .child(sample(
      slot(
        130.0,
        80.0,
        base(2)
          .transition(Transition::background_color().duration_ms(500))
          .transition(Transition::all().duration_ms(500).easing(Easing::EASE_IN_OUT))
          .hovered(|style| style.background(PaletteColor::Warning).rounded(30.0).size(120.0, 70.0)),
      ),
      "three properties on hover",
    ))
}

/// Tiles turned and scaled once, and turned without end.
fn transforms(palette: usize) -> Row {
  let forever =
    |keyframes: KeyframesId, duration: u64| Animation::new(keyframes).duration_ms(duration).linear().infinite();
  let card = |shown: Rect, caption: &str| sample(slot(120.0, 100.0, shown), caption);
  samples()
    .child(card(
      keyframe_tile(60.0, 0).transform(Transform2D::rotate_deg(45.0)),
      "rotate_deg(45.0)",
    ))
    .child(card(
      keyframe_tile(60.0, 1).transform(Transform2D::scale(1.4, 0.7)),
      "scale(1.4, 0.7)",
    ))
    .child(card(keyframe_tile(50.0, 2).animation(forever(SPIN, 2000)), "SPIN"))
    .child(card(keyframe_tile(50.0, 3).animation(forever(ROCK, 1000)), "ROCK"))
    .child(card(
      keyframe_tile(50.0, 0)
        .rounded(RadiusSize::Full)
        .animation(forever(SPIN_COLOR[palette], 3000)),
      "SPIN_COLOR",
    ))
}

impl Component for AnimationPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 6),
      property: ctx.signal(1),
      duration: ctx.signal(1600),
      easing: ctx.signal(4),
      delay: ctx.signal(0),
      iterations: ctx.signal(0),
      direction: ctx.signal(2),
      forwards: ctx.signal(false),
      plays: ctx.signal(0),
      eased: ctx.signal(false),
      ease_duration: ctx.signal(800),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let palette = palette(ctx);
    let rows = [
      (
        "animation",
        "Animation",
        "",
        "On an element: plays keyframes registered on the tree, from when the element is mounted. An element may \
         have several, of different keyframes.",
      ),
      (
        "Animation::new",
        "impl Into<KeyframesId>",
        "",
        "The keyframes it plays, by the number they were registered under.",
      ),
      (
        "duration_ms",
        "u64",
        "300",
        "How long one iteration takes. duration takes a Duration.",
      ),
      (
        "delay_ms",
        "u64",
        "0",
        "How long it waits before it starts. delay takes a Duration.",
      ),
      (
        "easing",
        "Easing",
        "EASE",
        "How the time of each span between two frames is eased; a frame's own easing is over it. linear() is \
         Easing::Linear.",
      ),
      (
        "iteration_count",
        "AnimationIterationCount",
        "Count(1.0)",
        "How many times it plays: a count, which may have a fraction, or Infinite, which infinite() gives.",
      ),
      (
        "direction",
        "AnimationDirection",
        "Normal",
        "Forward, backward, or each way in turn. alternate() is Alternate.",
      ),
      (
        "fill_mode",
        "AnimationFillMode",
        "None",
        "What the element shows before it starts and after it ends: its own style, or the pose the animation \
         starts or stops on.",
      ),
      (
        "transition",
        "Transition",
        "",
        "On an element: runs a property from its old value to its new one when it changes, in a state or by a \
         render. Transition::all() runs every property that changes.",
      ),
      (
        "hovered",
        "impl FnOnce(Style) -> Style",
        "",
        "The style under the pointer, over the element's own: what a transition runs to on hover and back from.",
      ),
      (
        "register_keyframes",
        "Keyframes",
        "",
        "On the tree, once: the frames of a sequence, at offsets from 0 to 1, each with the values it sets and an \
         easing of its own.",
      ),
      (
        "register_keyframes_in_phase",
        "Keyframes",
        "",
        "The same, with every run on the process's clock: elements mounted at different moments show the same pose.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Animation"],
        title: "Animation",
        description: "Transitions and keyframe animations: a transition runs a property from its old value to its \
                      new one when it changes; a keyframe animation plays a sequence registered on the tree.",
        import: Some("use lurq::animation::{Animation, Easing, Keyframes, KeyframesId, Transition};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A tile that plays one sequence of keyframes as the props say: which property it runs, how long, with what \
           easing, after what delay, how many times and which way. Press the tile to play it again from its start.",
          playground,
        ),
        Section::new(
          "Transitions on hover",
          "Each box has a style under the pointer and a transition that runs to it and back. Move the pointer over \
           them: a color, a border, a radius, the opacity, the width, and all of them at once.",
          self.frame.demo(
            0,
            hover_transitions(),
            ".transition(Transition::background_color().duration_ms(400))",
            &[
              "Rect::new(80.0, 60.0)",
              "    .background(PaletteColor::Chart1)",
              "    .transition(Transition::background_color().duration_ms(400))",
              "    .hovered(|s| s.background(PaletteColor::Destructive))",
              "",
              "// Several properties: one transition for the color, one for the rest.",
              "Rect::new(80.0, 60.0)",
              "    .transition(Transition::background_color().duration_ms(400).easing(Easing::EASE_IN_OUT))",
              "    .transition(Transition::all().duration_ms(500).easing(Easing::EASE_OUT))",
              "    .hovered(|s| {",
              "        s.background(PaletteColor::Destructive)",
              "            .rounded(28.0)",
              "            .border_inside(3.0, PaletteColor::Success)",
              "            .size(130.0, 70.0)",
              "    })",
            ],
          ),
        ),
        Section::new(
          "Easing",
          "The same transition of the width with each easing. Move the pointer over a bar, or turn the switch to run \
           them all at once and compare them; the slider sets how long they take.",
          self.frame.demo(
            1,
            self.easings(),
            ".easing(Easing::EASE_IN_OUT)",
            &[
              "Rect::new(Dimension::Pct(30.0), Dimension::Pct(100.0))",
              "    .transition(Transition::all().duration_ms(800).easing(Easing::EASE_IN_OUT))",
              "    .hovered(|s| s.width(Dimension::Pct(100.0)))",
              "",
              "// The others: Easing::Linear, Easing::EASE, Easing::EASE_IN, Easing::EASE_OUT,",
              "// Easing::Steps { count: 6, position: StepPosition::JumpEnd },",
              "// Easing::Spring { velocity: 0.0 }",
            ],
          ),
        ),
        Section::new(
          "Keyframe animations",
          "Every sequence the page registers on the tree, each playing without end: an opacity, a background, an \
           offset, a size, and a transform alone and with a background.",
          self.frame.demo(
            2,
            keyframes(palette),
            "Animation::new(PULSE).duration_ms(2000).linear().infinite()",
            &[
              "const PULSE: KeyframesId = KeyframesId::new(1);",
              "",
              "tree.register_keyframes(",
              "    Keyframes::new(PULSE)",
              "        .frame(0.0, |f| { f.set(AnimatableProperty::Opacity, 1.0); })",
              "        .frame(0.5, |f| { f.set(AnimatableProperty::Opacity, 0.3); })",
              "        .frame(1.0, |f| { f.set(AnimatableProperty::Opacity, 1.0); }),",
              ");",
              "",
              "Rect::new(64.0, 64.0)",
              "    .animation(Animation::new(PULSE).duration_ms(2000).linear().infinite())",
              "",
              "// A frame's own easing eases the span that ends at it:",
              "Keyframes::new(SLIDE_BOUNCE)",
              "    .frame(0.0, |f| { f.set(AnimatableProperty::OffsetX, 0.0); })",
              "    .frame(0.5, |f| { f.set(AnimatableProperty::OffsetX, 120.0).easing(Easing::EASE_OUT); })",
              "    .frame(1.0, |f| { f.set(AnimatableProperty::OffsetX, 0.0).easing(Easing::EASE_IN); })",
            ],
          ),
        ),
        Section::new(
          "Opacity",
          "The same tile at five opacities. An opacity is painted only: the tile keeps its place and its size.",
          self.frame.demo(
            3,
            opacities(),
            ".opacity(0.5)",
            &["Rect::new(60.0, 60.0).background(PaletteColor::Chart1).opacity(0.5)"],
          ),
        ),
        Section::new(
          "Together on one node",
          "A keyframe animation and a transition on the same element: the animation plays, and the transition \
           answers the pointer. Move the pointer over each.",
          self.frame.demo(
            4,
            combined(palette),
            ".animation(..).transition(..).hovered(..)",
            &[
              "Rect::new(100.0, 60.0)",
              "    .animation(Animation::new(PULSE).duration_ms(3000).linear().infinite())",
              "    .transition(Transition::background_color().duration_ms(300))",
              "    .hovered(|s| s.background(PaletteColor::Destructive))",
            ],
          ),
        ),
        Section::new(
          "Transforms",
          "A transform turns, scales or skews what is painted about the element's middle, and lays nothing out: a \
           tile turned or scaled keeps its place, once or in a keyframe animation of the Transform property.",
          self.frame.demo(
            5,
            transforms(palette),
            ".transform(Transform2D::rotate_deg(45.0))",
            &[
              "Rect::new(60.0, 60.0).transform(Transform2D::rotate_deg(45.0))",
              "Rect::new(60.0, 60.0).transform(Transform2D::scale(1.4, 0.7))",
              "",
              "// A full turn is a Decomposed: rotate_deg(360.0) is the matrix it started from.",
              "tree.register_keyframes(",
              "    Keyframes::new(SPIN)",
              "        .frame(0.0, |f| { f.set(AnimatableProperty::Transform, Decomposed::IDENTITY); })",
              "        .frame(1.0, |f| {",
              "            f.set(AnimatableProperty::Transform, Decomposed::IDENTITY.with_rotate(TAU));",
              "        }),",
              ");",
              "Rect::new(50.0, 50.0).animation(Animation::new(SPIN).duration_ms(2000).linear().infinite())",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an element is given to move, what an animation is built of, and what the tree is told once.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
