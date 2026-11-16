//! The playground of a component's page: the component on a stage, a control
//! for each of its props beside it, and under it the Rust that builds it,
//! what it did and what it is built of.
//!
//! A page keeps a [`Playground`] and says, each time it renders, what is on
//! the stage and what the panel has ([`Spec`]). The rest is the same on every
//! page: the switch that draws the component in a state with nothing pointing
//! at it, its measured size, the count of what it did, the tabs and Copy.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicU32, Ordering},
};

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{BorderSize, Breakpoint, Density, PaletteColor, RadiusSize, Theme, TypographyStyle},
    window::Appearance,
  },
  components::{Column, Rect, Row, ScrollBoth, ScrollHorizontal, TextInput, TextOverflow},
  core::{ElementRef, Signal, SignalValue},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, border::Border},
  ui::{
    Button, ButtonSize, ButtonVariant, Interaction, SegmentedControlPart, Switch, Tab, Tabs, TabsVariant, Toggle, icons,
  },
};

use crate::kit::{
  FILL, PANEL_ROOM, PHONE, code_block, icon, icon_tool, line, mono, name, plain_block, quiet, segmented, select,
  set_panel_room, stage_padding, text, tool,
};

/// The least width of the card with the panel beside the stage: the panel, and a stage whose bar holds the
/// switch of states and its tools.
const BESIDE: f32 = 780.0;

/// The states a component is drawn in with nothing pointing at it.
const FORCED: [(&str, Option<Interaction>); 4] = [
  ("Default", None),
  ("Hover", Some(Interaction::Hovered)),
  ("Pressed", Some(Interaction::Pressed)),
  ("Focus", Some(Interaction::FocusVisible)),
];

/// How many of the latest things a component did are kept.
const KEPT: usize = 50;
/// How high what a tab shows is at least: five lines of code, as the design draws it.
const OUTPUT: f32 = 150.0;
/// How high the bar over the stage is, and the bar of the tabs under it.
const TOOLBAR: f32 = 52.0;
/// Under this width a card is as narrow as a phone's page.
const NARROW: f32 = 480.0;
/// How high the bar of the tabs under the stage is at the default density: as high as its tabs, whose line
/// under the selected one is at their foot.
const TABS: f32 = 44.0;
/// How high the stage is, with the row of facts under the canvas.
const STAGE: f32 = 418.0;
/// How high the row of facts under the canvas is.
const FACTS: f32 = 36.0;
/// How wide the panel of props is beside the stage.
const PANEL: f32 = 320.0;
/// How wide the card of the last playground laid out was, as the bits of an `f32`: 0 until one was.
static LAST_ROOM: AtomicU32 = AtomicU32::new(0);

/// How high the bar of the tabs is: as high as the tabs, which follow the density as a row does.
fn tabs_height() -> f32 {
  let theme = lurq::ui::current_theme();
  TABS + theme.density_value(theme.density()) - theme.density_value(Density::Default)
}

/// The room a control of the panel has in a card `room` wide: the panel's beside the stage, and the card's less
/// its edges and the panel's padding under it.
fn panel_room(room: f32) -> f32 {
  if room <= 0.0 || room >= BESIDE {
    PANEL_ROOM
  } else {
    (room - 2.0 - 32.0).max(0.0)
  }
}

/// What a page keeps between renders for its playground.
pub struct Playground {
  forced: Signal<usize>,
  tab: Signal<usize>,
  count: Signal<u32>,
  /// What the component did, the latest first, a line each.
  events: Signal<String>,
  /// What holds the component on the stage, measured.
  anchor: ElementRef,
  measured: Signal<String>,
  /// The selects of the panel: what each hangs its list from, and whether it is open.
  selects: Vec<(ElementRef, Signal<bool>)>,
  /// The card, and how wide it is, measured: the panel stands beside the stage where there is room for both.
  card: ElementRef,
  room: Signal<f32>,
  /// What the panel holds, and how high it is, measured: beside a panel longer than the stage, what a tab shows
  /// is as high as the room that leaves under the stage.
  panel: ElementRef,
  panel_height: Signal<f32>,
  /// Whether the stage is in the appearance the page is not in.
  other: Signal<bool>,
  /// The theme of the stage while it is: the design's, in the other appearance.
  opposite: Theme,
}

/// What builds the component of a playground again, as its props were: for a window of its own.
pub type Make = Arc<dyn Fn() -> Element + Send + Sync>;

/// The props of the window a playground opens: what builds what it shows.
#[derive(Clone, lurq::DevtoolsInspectable)]
struct DetachedProps {
  #[devtools_ignore]
  make: Make,
}

impl PartialEq for DetachedProps {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.make, &other.make)
  }
}

/// A window with the component of a playground alone on the canvas, to see it at any size of window.
struct Detached;

impl Component for Detached {
  type Props = DetachedProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let make = ctx.props::<DetachedProps>().make.clone();
    Column::new()
      .size(FILL, FILL)
      .padding(32.0)
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .background(PaletteColor::Canvas)
      .foreground(PaletteColor::Foreground)
      .child(make())
  }
}

/// What is on the stage, built when the stage is: in the stage's theme, which is not the page's while the stage is
/// in the other appearance. A component reads its theme when it becomes an element, and part of what it reads is
/// the theme's as it is then (the color a text input types in, a slider's track), so an element built by the page
/// would keep the page's colors on the stage.
pub struct Shown<'a> {
  build: Option<Box<dyn FnOnce(&mut Ctx) -> Element + 'a>>,
  /// Whether the scene measures the component itself: see [`Shown::measured`].
  measured: bool,
}

impl<'a> Shown<'a> {
  /// A component, or a scene with nothing of the design system built in it yet: it becomes an element on the stage.
  pub fn of(shown: impl Into<Element> + 'a) -> Self {
    Self {
      build: Some(Box::new(move |_| shown.into())),
      measured: false,
    }
  }

  /// A scene the page builds of more than one thing: built on the stage, with all that is in it.
  pub fn built(build: impl FnOnce(&mut Ctx) -> Element + 'a) -> Self {
    Self {
      build: Some(Box::new(build)),
      measured: false,
    }
  }

  /// The scene holds more than the component (a column that gives it a width, what a separator is between), and
  /// has put [`Playground::measure`] around the component in it: the size under the stage is the component's and
  /// not the scene's.
  pub fn measured(mut self) -> Self {
    self.measured = true;
    self
  }

  fn take(&mut self, ctx: &mut Ctx) -> Element {
    match self.build.take() {
      Some(build) => build(ctx),
      None => Row::new().into(),
    }
  }
}

/// What a page says of its playground when it renders.
pub struct Spec<'a> {
  /// What the component is called: the title of the window it is opened in.
  pub title: &'static str,
  /// The component, as its props make it.
  pub shown: Shown<'a>,
  /// What builds the component again as its props are now, with nothing listening to it: what a window of
  /// its own shows.
  pub make: Make,
  /// Whether the component is as wide as the stage has room for.
  pub fill: bool,
  /// Whether the component can be drawn in a state with nothing pointing at it: the switch over the stage.
  pub forceable: bool,
  /// What is said after the measured size: "radius 6".
  pub facts: String,
  /// What the component does that is counted: "on_click". Empty for a component that is read and not operated:
  /// nothing is counted.
  pub event: &'static str,
  /// The Rust that builds the component, a call to a line.
  pub code: Vec<String>,
  /// The parts the component is built of.
  pub tree: Vec<String>,
  /// The controls of the panel, one for each prop: [`prop`], [`flag`], [`divider`].
  pub controls: Vec<Element>,
  /// How many props are not what the component has when it says nothing.
  pub changed: usize,
  /// Puts every prop back to what the playground starts with.
  pub reset: Arc<dyn Fn() + Send + Sync>,
}

impl Playground {
  /// A playground whose panel has `selects` selects: call it where the page is created.
  pub fn new(ctx: &mut Ctx, selects: usize) -> Self {
    // In a window as narrow as a phone the card is the window less the page's edges, 16 at each side: known
    // before anything is laid out, so the first frame of the first page is the narrow form and not the wide
    // one cut off.
    if f32::from_bits(LAST_ROOM.load(Ordering::Relaxed)) <= 0.0 {
      let window = ctx.window().info().resolved_width;
      if window > 0.0 && window < PHONE {
        LAST_ROOM.store((window - 32.0).to_bits(), Ordering::Relaxed);
        set_panel_room(panel_room(window - 32.0));
      }
    }
    Self {
      forced: ctx.signal(0),
      tab: ctx.signal(0),
      count: ctx.signal(0),
      events: ctx.signal(String::new()),
      anchor: ElementRef::new(),
      measured: ctx.signal("0 × 0 px".to_owned()),
      selects: (0..selects).map(|_| (ElementRef::new(), ctx.signal(false))).collect(),
      card: ElementRef::new(),
      // A page opened after another starts with the width the last one measured: its card is as wide.
      room: ctx.signal(f32::from_bits(LAST_ROOM.load(Ordering::Relaxed))),
      panel: ElementRef::new(),
      panel_height: ctx.signal(0.0),
      other: ctx.signal(false),
      opposite: Theme::new(),
    }
  }

  /// The state the switch over the stage asks for: what the page gives its component's `force`.
  pub fn forced(&self) -> Option<Interaction> {
    FORCED[self.forced.get().min(FORCED.len() - 1)].1
  }

  /// The size of the component on the stage as it was last laid out: "144 × 36 px".
  pub fn measured(&self) -> String {
    self.measured.get()
  }

  /// The component of a scene that holds more than it, in the row that is measured: as wide and as high as the
  /// component makes itself where the scene puts it. The scene is given to the stage as [`Shown::measured`].
  pub fn measure(&self, component: impl Into<Element>) -> Row {
    Row::new()
      .ref_element(self.anchor.clone())
      .overflow_visible()
      .child(component)
  }

  /// What a handler of the component calls to say what it did: the line is
  /// counted, and goes to the top of the Events tab.
  pub fn recorder(&self) -> impl Fn(String) + Clone + Send + Sync + 'static {
    let (count, events) = (self.count.clone(), self.events.clone());
    move |what: String| {
      count.update(|count| *count += 1);
      let number = count.get();
      events.update(|log| {
        let earlier: Vec<&str> = log.lines().take(KEPT - 1).collect();
        *log = std::iter::once(format!("{what}  #{number}").as_str())
          .chain(earlier)
          .collect::<Vec<_>>()
          .join("\n");
      });
    }
  }

  /// Counts each change of a signal the component is bound to, and says it in the Events tab as `say` words
  /// it. A value set to what it already was is no change: a control that sets its value as it is built, or a
  /// Reset that puts back what is there, is not counted.
  pub fn watch<T: SignalValue + Clone + PartialEq + Send + Sync + 'static>(
    &self,
    ctx: &mut Ctx,
    signal: &Signal<T>,
    say: impl Fn(&T) -> String + Send + Sync + 'static,
  ) {
    let record = self.recorder();
    let last = Mutex::new(signal.get());
    ctx.watch(signal, move |value| {
      let mut last = last.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
      if *last != *value {
        *last = value.clone();
        record(say(value));
      }
    });
  }

  /// A select of the panel, the `index`th of those the playground was made with.
  pub fn select<T: Clone + PartialEq + Send + Sync + 'static>(
    &self,
    index: usize,
    options: &[(&str, T)],
    chosen: &T,
    choose: impl Fn(T) + Send + Sync + 'static,
  ) -> Element {
    let (anchor, open) = &self.selects[index];
    select(anchor, open, options, chosen, choose).width(FILL).into()
  }

  /// The bar over the stage: the state the component is drawn in, the appearance of the stage, and the
  /// component in a window of its own.
  fn toolbar(&self, ctx: &mut Ctx, spec: &Spec<'_>) -> Element {
    let forceable = spec.forceable;
    // A card as narrow as a phone has no room for the states beside the buttons: they go on a row of their own
    // under the bar, as wide as the card.
    let room = self.room.get();
    let narrow = forceable && room > 0.0 && room < NARROW;
    let mut bar = Row::new()
      .width(FILL)
      .height(TOOLBAR)
      .padding_left(16.0)
      .padding_right(12.0)
      .align_items(Alignment::Center)
      .justify(Justify::SpaceBetween);
    if !narrow {
      bar = bar.border_bottom(line());
    }
    let states = || {
      let forced = self.forced.clone();
      segmented(&FORCED, &self.forced(), move |interaction| {
        forced.set(FORCED.iter().position(|(_, held)| *held == interaction).unwrap_or(0))
      })
    };
    if narrow {
      bar = bar.child(quiet("state", 12.0));
    } else if forceable {
      bar = bar.child(
        Row::new()
          .spacing(10.0)
          .align_items(Alignment::Center)
          .child(quiet("state", 12.0))
          .child(states()),
      );
    } else {
      // A component with no event that is counted: one that is read, or one that answers the pointer and tells
      // the app nothing of it (a hover card, a drawer). The stage says what it counts, not what can be pressed.
      bar = bar.child(
        Row::new().flex(1.0).min_width(0.0).padding_right(12.0).child(
          quiet(
            if spec.event.is_empty() {
              "as the props make it: it tells the app of no event to count"
            } else {
              "live: what is on the stage answers the pointer and the keyboard"
            },
            12.0,
          )
          .text_overflow(TextOverflow::Elipsis),
        ),
      );
    }
    let other = self.other.clone();
    let (opener, make, title) = (ctx.window_opener(), spec.make.clone(), spec.title);
    let bar = bar.child(
      Row::new()
        .spacing(4.0)
        .flex_shrink(0.0)
        .align_items(Alignment::Center)
        .child(
          Toggle::new("Show the stage in the other appearance")
            .size(ButtonSize::Sm)
            .icon(icons::MOON)
            .icon_only(true)
            .on(self.other.get())
            .on_change(move |on| other.set(on)),
        )
        .child(
          Button::new("Open in a window of its own")
            .variant(ButtonVariant::Ghost)
            .size(ButtonSize::Sm)
            .icon(icons::EXTERNAL_LINK)
            .icon_only(true)
            .on_click(move |_| {
              let make = make.clone();
              opener.open(format!("{title} · lurq gallery"), 520, 360, move |app, tree| {
                tree.mount_root::<Detached>(app, DetachedProps { make });
              });
            }),
        ),
    );
    if narrow {
      Column::new()
        .width(FILL)
        .border_bottom(line())
        .child(bar)
        .child(
          Row::new()
            .width(FILL)
            .padding_horizontal(12.0)
            .padding_bottom(12.0)
            // As high as it is over a wide stage, so that a state is as easy to press: the names have less
            // room beside them, and share the card's width.
            .child(
              states()
                .full_width(true)
                .style(SegmentedControlPart::Segment, |s| s.padding_x(4.0)),
            ),
        )
        .into()
    } else {
      bar.into()
    }
  }

  /// The stage: the component on the canvas, with its measured size and the count of what it did under it.
  /// It is `room` wide: what is larger than it is scrolled to, and what is smaller stands in its middle.
  fn stage(&self, ctx: &mut Ctx, spec: &mut Spec<'_>, room: f32) -> Column {
    // What is on the stage is measured by this row, which cuts nothing off: a shadow, a ring or a badge on a
    // corner reaches past the component's box, and the stage has the room for it.
    let mut held = Row::new().overflow_visible();
    if !spec.shown.measured {
      held = held.ref_element(self.anchor.clone());
    }
    let held = held.child(spec.shown.take(ctx));
    // What scrolls has no width to fill: a component as wide as the stage is given the stage's, in a lane of
    // that width. The row that is measured is as wide as the component makes itself in the lane, so a component
    // that keeps a width of its own is measured at it and stands in the middle.
    let held: Element = if spec.fill {
      let lane = Column::new().align_items(Alignment::Center).child(held);
      if room > 0.0 {
        lane.width((room - 2.0 * stage_padding()).max(0.0)).into()
      } else {
        lane.width(FILL).into()
      }
    } else {
      held.into()
    };
    // The canvas is at least the stage, so what fits is in its middle, and as large as what is on it, so
    // what does not fit is reached by scrolling and nothing of it is cut.
    let canvas = Row::new()
      .padding_horizontal(stage_padding())
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .child(held);
    // Until the card was laid out once nothing says how wide the stage is: the canvas is the stage then, which
    // is where what fits stands on a canvas that scrolls too, so nothing moves when the width is known.
    let canvas: Element = if room > 0.0 {
      ScrollBoth::new(canvas.min_width(room).min_height(STAGE - FACTS))
        .key("stage")
        .width(FILL)
        .height(STAGE - FACTS)
        .into()
    } else {
      canvas.width(FILL).height(STAGE - FACTS).into()
    };
    let measured = if spec.facts.is_empty() {
      self.measured()
    } else {
      format!("{} · {}", self.measured(), spec.facts)
    };
    // The names of the handlers give way to the size, which is what the line is for: they end in an ellipsis
    // where there is no room for all of them.
    let counted = || {
      Row::new()
        .spacing(6.0)
        .min_width(0.0)
        .flex_shrink(1.0)
        .align_items(Alignment::Center)
        .child(icon(icons::HAND_POINTER, 14.0))
        .child(
          Row::new()
            .min_width(0.0)
            .flex_shrink(1.0)
            .child(mono(spec.event, 12.0).nowrap().text_overflow(TextOverflow::Elipsis)),
        )
        .child(mono(&format!("× {}", self.count.get()), 12.0).nowrap().flex_shrink(0.0))
    };
    let stage = Column::new()
      .width(FILL)
      .background(PaletteColor::Canvas)
      .foreground(PaletteColor::Foreground)
      .child(canvas);
    // A card as narrow as a phone has no room for the size and the count on one line: the size is on lines of
    // its own, as many as it takes, and the count under it; the stage is as much higher.
    if room > 0.0 && room < NARROW {
      let mut facts = Column::new()
        .width(FILL)
        .min_height(FACTS)
        .padding_horizontal(16.0)
        .padding_vertical(8.0)
        .spacing(4.0)
        .justify(Justify::Center)
        .foreground(PaletteColor::MutedForeground)
        .child(mono(&measured, 12.0).width(FILL));
      if !spec.event.is_empty() {
        facts = facts.child(counted().width(FILL));
      }
      return stage.min_height(STAGE).child(facts);
    }
    stage.height(STAGE).child(
      Row::new()
        .width(FILL)
        .height(FACTS)
        .padding_horizontal(16.0)
        .align_items(Alignment::Center)
        .justify(Justify::SpaceBetween)
        .foreground(PaletteColor::MutedForeground)
        .spacing(16.0)
        .child(mono(&measured, 12.0).nowrap().flex_shrink(0.0))
        .child(if spec.event.is_empty() {
          // A component that is read and not operated has nothing to count.
          Row::new()
        } else {
          counted()
        }),
    )
  }

  /// The tabs under the stage, and what the chosen one shows, `height` high.
  fn output(&self, spec: &Spec<'_>, height: f32, narrow: bool) -> Column {
    let events = self.events.get();
    let shown = self.tab.get();
    let tab = self.tab.clone();
    let count = self.count.get();
    let tabs = Tabs::controlled(
      shown,
      [
        Tab::new(0, "Code"),
        if count > 0 {
          Tab::new(1, "Events").count(count)
        } else {
          Tab::new(1, "Events")
        },
        Tab::new(2, "Layout tree"),
      ],
      move |index| tab.set(*index),
    )
    .variant(TabsVariant::Line)
    .label("What the stage shows");
    // In a card as narrow as a phone Copy is its icon alone, which leaves the tabs their room; a count of
    // what the component did can still make them wider than it, and then they scroll sideways in it.
    let tabs: Element = if narrow {
      ScrollHorizontal::new(tabs).key("output-tabs").width(FILL).into()
    } else {
      tabs.into()
    };
    let copied = match shown {
      0 => spec.code.join("\n"),
      1 => events.clone(),
      _ => spec.tree.join("\n"),
    };
    // As high as it is told, whatever it shows: a longer log or a longer builder scrolls in it, and the card
    // keeps its height.
    let body: Element = match shown {
      0 => code_block(&spec.code),
      1 if events.is_empty() => plain_block(&["Nothing yet: use what is on the stage.".to_owned()]),
      1 => plain_block(&events.lines().map(str::to_owned).collect::<Vec<_>>()),
      _ => plain_block(&spec.tree),
    }
    .min_height(height)
    .max_height(height)
    .key(format!("output-{shown}"))
    .into();
    Column::new()
      .width(FILL)
      .child(
        Row::new()
          .width(FILL)
          .height(tabs_height())
          .padding_left(16.0)
          .padding_right(8.0)
          .align_items(Alignment::Center)
          .justify(Justify::SpaceBetween)
          .border_top(line())
          .border_bottom(line())
          // The tabs take the room there is and Copy keeps its own: tabs are as wide as what holds them, and
          // beside them in a row they pushed Copy out of the bar.
          .child(Row::new().flex(1.0).min_width(0.0).child(tabs))
          .child(Row::new().flex_shrink(0.0).child({
            let copy = move || {
              lurq::clipboard::copy_to_clipboard(&copied);
            };
            if narrow {
              icon_tool("Copy", icons::COPY, copy)
            } else {
              tool("Copy", icons::COPY, copy)
            }
          })),
      )
      .child(body)
  }

  /// The panel beside the stage: how many props were changed, Reset, and the controls.
  fn panel(&self, spec: &mut Spec<'_>) -> Column {
    let reset = {
      let (reset, count, events) = (spec.reset.clone(), self.count.clone(), self.events.clone());
      move || {
        reset();
        count.set(0);
        events.set(String::new());
      }
    };
    let mut controls = Column::new().width(FILL).padding(16.0).spacing(16.0);
    for control in std::mem::take(&mut spec.controls) {
      controls = controls.child(control);
    }
    Column::new()
      .child(
        Row::new()
          .width(FILL)
          .height(52.0)
          .padding_left(16.0)
          .padding_right(8.0)
          .align_items(Alignment::Center)
          .justify(Justify::SpaceBetween)
          .border_bottom(line())
          .child(
            Row::new()
              .spacing(8.0)
              .align_items(Alignment::Center)
              .child(text("Props", 14.0, FontWeight::SemiBold))
              .child(quiet(&format!("{} changed", spec.changed), 11.0)),
          )
          .child(tool("Reset", icons::REFRESH, move || reset())),
      )
      .child(controls)
  }

  /// The playground: the stage with its tabs, and the panel beside it, or under it in a narrow window.
  pub fn card(&self, ctx: &mut Ctx, mut spec: Spec<'_>) -> Element {
    // What is on the stage says how large it is whenever that changes.
    let measured = self.measured.clone();
    self.anchor.observe_rect(move |rect| {
      let size = format!("{} × {} px", rect.width.round(), rect.height.round());
      if measured.get() != size {
        measured.set(size);
      }
    });
    let room = self.room.clone();
    self.card.observe_rect(move |rect| {
      if (room.get() - rect.width).abs() >= 0.5 {
        // The page builds the controls of the panel before the card: the render this change asks for has the
        // panel's room when it does.
        set_panel_room(panel_room(rect.width));
        room.set(rect.width);
        LAST_ROOM.store(rect.width.to_bits(), Ordering::Relaxed);
      }
    });
    // The window says whether the panel is beside the stage until the card was laid out once; from then
    // on the card's own width does: a wide window with a narrow column for the page is not room for both.
    let room = self.room.get();
    let wide = if room > 0.0 {
      room >= BESIDE
    } else {
      ctx.breakpoint() >= Some(Breakpoint::Lg)
    };
    set_panel_room(panel_room(room));
    let toolbar = self.toolbar(ctx, &spec);
    // The stage is the card less its edges, and less the panel and the line before it where that is beside it.
    let stage_room = match (room > 0.0, wide) {
      (false, _) => 0.0,
      (true, true) => (room - 2.0 - PANEL - 1.0).max(0.0),
      (true, false) => (room - 2.0).max(0.0),
    };
    // The stage in the appearance the page is not in: a theme of its own, for it and what is on it, which is
    // built in that theme.
    let stage: Element = if self.other.get() {
      self
        .opposite
        .set_preferred_appearance(Some(match ctx.theme().appearance() {
          Appearance::Dark => Appearance::Light,
          _ => Appearance::Dark,
        }));
      let opposite = self.opposite.clone();
      ctx.in_theme(&opposite, |ctx| self.stage(ctx, &mut spec, stage_room))
    } else {
      self.stage(ctx, &mut spec, stage_room).into()
    };
    // What the panel holds is measured before the paint, so the first frame shown is the settled one.
    let panel_height = self.panel_height.clone();
    self.panel.observe_rect_before_paint(move |rect| {
      if (panel_height.get() - rect.height).abs() >= 0.5 {
        panel_height.set(rect.height);
      }
    });
    // Beside a panel longer than the stage with its tabs, what a tab shows takes the room that is left under the
    // stage, so the code is not five lines over an empty half of the card.
    let output = if wide {
      (self.panel_height.get() - TOOLBAR - STAGE - tabs_height()).max(OUTPUT)
    } else {
      OUTPUT
    };
    let preview =
      Column::new()
        .child(toolbar)
        .child(stage)
        .child(self.output(&spec, output, room > 0.0 && room < NARROW));
    // The panel's own height is measured inside the column the row stretches: the stretched one is as high as
    // the stage's side, which would only ever grow.
    let panel = Column::new().child(self.panel(&mut spec).width(FILL).ref_element(self.panel.clone()));
    let card = Column::new()
      .width(FILL)
      .clip()
      .rounded(RadiusSize::Lg)
      .background(PaletteColor::Card)
      .foreground(PaletteColor::CardForeground)
      .border(line())
      .ref_element(self.card.clone());
    if wide {
      card
        .child(
          Row::new()
            .width(FILL)
            .align_items(Alignment::Stretch)
            .child(preview.flex(1.0))
            .child(panel.width(PANEL).border_left(line())),
        )
        .into()
    } else {
      card
        .child(preview.width(FILL))
        .child(panel.width(FILL).border_top(line()))
        .into()
    }
  }
}

/// A control of the panel under the name of its prop and its type.
pub fn prop(label: &str, kind: &str, control: impl Into<Element>) -> Element {
  Column::new()
    .width(FILL)
    .spacing(6.0)
    .child(
      Row::new()
        .width(FILL)
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(name(label, 12.0).nowrap().flex_shrink(0.0))
        // The type stands at the panel's end, and one longer than the room beside the name goes on to a
        // second line there.
        .child(
          Column::new()
            .flex(1.0)
            .min_width(0.0)
            .align_items(Alignment::End)
            .child(quiet(kind, 11.0)),
        ),
    )
    .child(control)
    .into()
}

/// A prop that is on or off: its name, and a switch.
pub fn flag(label: &str, value: &Signal<bool>) -> Element {
  let set = value.clone();
  // A long label wraps under itself and the switch keeps its place at the end: nothing of a row leaves the panel.
  Row::new()
    .width(FILL)
    .min_height(24.0)
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(
      Row::new()
        .flex(1.0)
        .min_width(0.0)
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(name(label, 12.0).flex_shrink(1.0))
        .child(quiet("bool", 11.0).nowrap().flex_shrink(0.0)),
    )
    .child(
      Row::new().flex_shrink(0.0).child(
        Switch::new(label)
          .label_hidden(true)
          .on(value.get())
          .on_change(move |on| set.set(on)),
      ),
    )
    .into()
}

/// A prop that is text: a field bound to it.
pub fn field(value: &Signal<String>) -> Element {
  let body = lurq::ui::current_theme().typography_style(TypographyStyle::Body);
  Row::new()
    .width(FILL)
    .height(36.0)
    .padding_horizontal(12.0)
    .align_items(Alignment::Center)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Background)
    .border(Border::inside(BorderSize::Sm, PaletteColor::Input))
    .child(
      TextInput::styled(value.clone(), body)
        .flex(1.0)
        .height(FILL)
        .tab_index(0),
    )
    .into()
}

/// A line between the props that are chosen and those that are on or off.
pub fn divider() -> Element {
  Rect::new(FILL, 1.0).background(PaletteColor::Border).into()
}
