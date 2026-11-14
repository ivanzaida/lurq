//! The page of the timeline: its playground, the board's history of a project, a deploy with its marks and tones, the
//! activity of a document with avatars and content under its items, a log, a history down the middle, and an order's
//! steps across.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::Text,
  core::Signal,
  node::Element,
  ui::{
    Avatar, Button, ButtonSize, ButtonVariant, Card, Orientation, Timeline, TimelineAlign, TimelineItem, TimelinePart,
    TimelineSize, TimelineVariant,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, segmented_fill},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// What stands for the items' marks in the playground.
#[derive(Clone, Copy, PartialEq)]
enum Marks {
  Dots,
  Icons,
  Avatars,
}

const MARKS: [(&str, Marks); 3] = [
  ("Dots", Marks::Dots),
  ("Icons", Marks::Icons),
  ("Avatars", Marks::Avatars),
];

const ORIENTATIONS: [(&str, Orientation); 2] = [
  ("Vertical", Orientation::Vertical),
  ("Horizontal", Orientation::Horizontal),
];

const SIZES: [(&str, TimelineSize); 2] = [("Md", TimelineSize::Md), ("Sm", TimelineSize::Sm)];

const ALIGNS: [(&str, TimelineAlign); 3] = [
  ("Start", TimelineAlign::Start),
  ("Alternate", TimelineAlign::Alternate),
  ("TimeColumn", TimelineAlign::TimeColumn),
];

/// The names of a list of choices with their places, for a segmented control of the panel.
fn places<T>(choices: &[(&'static str, T)]) -> Vec<(&'static str, usize)> {
  choices
    .iter()
    .enumerate()
    .map(|(index, (name, _))| (*name, index))
    .collect()
}

/// The board's timeline: the history of a project, the oldest item to come.
fn board() -> Timeline {
  Timeline::new([
    TimelineItem::new("Released 0.60")
      .description("Blur and frosted glass land in the renderer.")
      .time("Nov 10"),
    TimelineItem::new("Merged node-blur")
      .description("Blurs follow transforms and rounded clips.")
      .time("Nov 8"),
    TimelineItem::new("Opened pull request")
      .description("Blur a node and its subtree as one layer.")
      .time("Nov 3")
      .upcoming(true),
  ])
  .label("History")
  .style(TimelinePart::Root, |s| s.width(300.0))
}

/// What the playground's panel says of the timeline.
#[derive(Clone, Copy)]
struct Props {
  orientation: Orientation,
  size: TimelineSize,
  align: TimelineAlign,
  marks: Marks,
  descriptions: bool,
  times: bool,
  long: bool,
  failed: bool,
  current: bool,
  pending: bool,
  upcoming: bool,
  content: bool,
  action: bool,
  select: bool,
  more: bool,
  loading: bool,
}

pub struct TimelinePage {
  frame: Frame,
  playground: Playground,
  /// Which of [`ORIENTATIONS`], [`SIZES`], [`ALIGNS`] and [`MARKS`] are chosen.
  orientation: Signal<usize>,
  size: Signal<usize>,
  align: Signal<usize>,
  marks: Signal<usize>,
  descriptions: Signal<bool>,
  times: Signal<bool>,
  long: Signal<bool>,
  failed: Signal<bool>,
  current: Signal<bool>,
  pending: Signal<bool>,
  upcoming: Signal<bool>,
  content: Signal<bool>,
  action: Signal<bool>,
  select: Signal<bool>,
  more: Signal<bool>,
  loading: Signal<bool>,
}

/// The playground's timeline: a deploy, as `props` make it, each step told by its place when it is pressed, and
/// telling `record` what was pressed.
fn deploy(props: Props, record: Option<Arc<dyn Fn(String) + Send + Sync>>) -> Timeline<usize> {
  let steps: [(&str, &str, &str, &str, &str); 4] = [
    (
      "Built",
      "Compiled in release mode with LTO.",
      "14:02",
      "box",
      "Ivan Zaida",
    ),
    (
      "Tested",
      "1,174 tests passed on three platforms.",
      "14:09",
      "check",
      "Olena Koval",
    ),
    (
      "Deploying",
      "Rolling out to the first region.",
      "14:15",
      "send",
      "Taras Bondar",
    ),
    ("Released", "Every region serves 0.61.", "", "star", "Mariia Shevchuk"),
  ];
  let items = steps
    .iter()
    .enumerate()
    .map(|(index, (title, description, time, icon, who))| {
      let mut item = TimelineItem::new(title).value(index);
      if props.descriptions {
        let text = if props.long && index == 1 {
          "1,174 tests passed on Windows, macOS and Linux, among them the tests of the renderer against the board's \
         pictures, the tests of the layout of every component at every density, and the tests of what tooling \
         reads."
        } else {
          description
        };
        item = item.description(text);
      }
      if props.times && !time.is_empty() {
        item = item.time(time);
      }
      item = match props.marks {
        Marks::Dots => item,
        Marks::Icons => item.icon(*icon),
        Marks::Avatars => item.media(Avatar::new(who).size("xs")),
      };
      if props.failed && index == 1 {
        item = item.variant(TimelineVariant::Destructive);
      }
      if index == 2 {
        item = item.current(props.current).pending(props.pending);
      }
      if index == 3 {
        item = item.upcoming(props.upcoming);
      }
      if props.content && index == 0 {
        item = item.child(
          Card::new()
            .description("lurq-0.61.0-x86_64-pc-windows-msvc.zip, 18.4 MB")
            .width(lurq::ui::Length::Fill),
        );
      }
      if props.action && index == 2 {
        item = item.action(
          Button::new("Cancel")
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Sm),
        );
      }
      item
    });
  let mut timeline = Timeline::new(items)
    .label("Deploy")
    .orientation(props.orientation)
    .size(props.size)
    .align(props.align);
  if props.orientation.is_vertical() {
    let width = if props.align == TimelineAlign::Alternate {
      520.0
    } else {
      340.0
    };
    timeline = timeline.style(TimelinePart::Root, move |s| s.width(width));
  }
  if props.select {
    let record = record.clone();
    timeline = timeline.on_select(move |step: &usize| {
      if let Some(record) = &record {
        record(format!("on_select({step})"));
      }
    });
  }
  if props.more {
    timeline = timeline.loading(props.loading).on_load_more(move || {
      if let Some(record) = &record {
        record("on_load_more()".to_owned());
      }
    });
  }
  timeline
}

impl TimelinePage {
  fn props(&self) -> Props {
    Props {
      orientation: ORIENTATIONS[self.orientation.get().min(ORIENTATIONS.len() - 1)].1,
      size: SIZES[self.size.get().min(SIZES.len() - 1)].1,
      align: ALIGNS[self.align.get().min(ALIGNS.len() - 1)].1,
      marks: MARKS[self.marks.get().min(MARKS.len() - 1)].1,
      descriptions: self.descriptions.get(),
      times: self.times.get(),
      long: self.long.get(),
      failed: self.failed.get(),
      current: self.current.get(),
      pending: self.pending.get(),
      upcoming: self.upcoming.get(),
      content: self.content.get(),
      action: self.action.get(),
      select: self.select.get(),
      more: self.more.get(),
      loading: self.loading.get(),
    }
  }

  /// The playground: a deploy as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let record = self.playground.recorder();
    let record: Arc<dyn Fn(String) + Send + Sync> = Arc::new(record);
    let shown = deploy(props, Some(record)).force(if props.select { self.playground.forced() } else { None });
    let make = move || -> Element { deploy(props, None).into() };

    let mut code = vec!["Timeline::new([".to_owned()];
    let mark = match props.marks {
      Marks::Dots => "",
      Marks::Icons => ".icon(\"box\")",
      Marks::Avatars => ".media(Avatar::new(\"Ivan Zaida\").size(\"xs\"))",
    };
    let mut first = format!("    TimelineItem::new(\"Built\").value(0){mark}");
    if props.descriptions {
      first.push_str(".description(\"Compiled in release mode with LTO.\")");
    }
    if props.times {
      first.push_str(".time(\"14:02\")");
    }
    if props.content {
      first.push_str(".child(Card::new().description(\"lurq-0.61.0-..zip, 18.4 MB\"))");
    }
    code.push(format!("{first},"));
    if props.failed {
      code.push("    TimelineItem::new(\"Tested\").variant(TimelineVariant::Destructive),".to_owned());
    } else {
      code.push("    TimelineItem::new(\"Tested\"),".to_owned());
    }
    let mut third = "    TimelineItem::new(\"Deploying\")".to_owned();
    if props.current {
      third.push_str(".current(true)");
    }
    if props.pending {
      third.push_str(".pending(true)");
    }
    if props.action {
      third.push_str(".action(Button::new(\"Cancel\"))");
    }
    code.push(format!("{third},"));
    code.push(format!(
      "    TimelineItem::new(\"Released\"){},",
      if props.upcoming { ".upcoming(true)" } else { "" }
    ));
    code.push("])".to_owned());
    code.push(".label(\"Deploy\")".to_owned());
    if !props.orientation.is_vertical() {
      code.push(".orientation(Orientation::Horizontal)".to_owned());
    }
    if props.size == TimelineSize::Sm {
      code.push(".size(TimelineSize::Sm)".to_owned());
    }
    if props.align != TimelineAlign::Start {
      code.push(format!(".align(TimelineAlign::{})", ALIGNS[self.align.get().min(2)].0));
    }
    if props.select {
      code.push(".on_select(|step: &usize| /* open it */)".to_owned());
    }
    if props.more {
      code.push(".on_load_more(|| /* fetch the next page */)".to_owned());
      if props.loading {
        code.push(".loading(true)".to_owned());
      }
    }

    let wide = props.marks != Marks::Dots;
    let tree = vec![
      format!("root        {}   items one under another", self.playground.measured()),
      "└─ item     12 between the rail and the content".to_owned(),
      format!(
        "   ├─ rail     {}",
        if wide {
          "24: the mark, a line of 1 to 4 over the next"
        } else {
          "12: 4 down, a dot of 10, 4, a line of 1"
        }
      ),
      "   └─ content  2 apart, 20 under it (none under the last)".to_owned(),
      "      ├─ title        14 at 500, wraps; the action after the content".to_owned(),
      "      ├─ description  13, muted, lines of 1.4".to_owned(),
      "      ├─ time         mono 11, muted".to_owned(),
      "      └─ body         8 under the time".to_owned(),
    ];

    let segmented = |signal: &Signal<usize>, choices: Vec<(&'static str, usize)>| {
      let choose = signal.clone();
      segmented_fill(&choices, &signal.get(), move |index| choose.set(index))
    };
    let mut controls = vec![
      prop(
        "orientation",
        "Orientation",
        segmented(&self.orientation, places(&ORIENTATIONS)),
      ),
      prop("size", "TimelineSize", segmented(&self.size, places(&SIZES))),
    ];
    if props.orientation.is_vertical() {
      controls.push(prop("align", "TimelineAlign", segmented(&self.align, places(&ALIGNS))));
    }
    controls.extend([
      divider(),
      prop("marks", "icon / media", {
        let choose = self.marks.clone();
        let choices: Vec<(&str, usize)> = MARKS
          .iter()
          .enumerate()
          .map(|(index, (name, _))| (*name, index))
          .collect();
        segmented_fill(&choices, &self.marks.get(), move |index| choose.set(index))
      }),
      divider(),
      flag("description", &self.descriptions),
      flag("time", &self.times),
    ]);
    if props.descriptions {
      controls.push(flag("a long description", &self.long));
    }
    controls.extend([
      divider(),
      flag("variant: Destructive (Tested)", &self.failed),
      flag("current (Deploying)", &self.current),
      flag("pending (Deploying)", &self.pending),
      flag("upcoming (Released)", &self.upcoming),
      divider(),
      flag("child: a card (Built)", &self.content),
      flag("action (Deploying)", &self.action),
      flag("on_select (the timeline's)", &self.select),
      divider(),
      flag("on_load_more", &self.more),
    ]);
    if props.more {
      controls.push(flag("loading", &self.loading));
    }
    let defaults = Props {
      orientation: Orientation::Vertical,
      size: TimelineSize::Md,
      align: TimelineAlign::Start,
      marks: Marks::Dots,
      descriptions: true,
      times: true,
      long: false,
      failed: false,
      current: true,
      pending: true,
      upcoming: true,
      content: false,
      action: false,
      select: false,
      more: false,
      loading: false,
    };
    let changed = [
      props.orientation != defaults.orientation,
      props.size != defaults.size,
      props.align != defaults.align,
      props.marks != defaults.marks,
      props.descriptions != defaults.descriptions,
      props.times != defaults.times,
      props.long != defaults.long,
      props.failed != defaults.failed,
      props.current != defaults.current,
      props.pending != defaults.pending,
      props.upcoming != defaults.upcoming,
      props.content != defaults.content,
      props.action != defaults.action,
      props.select != defaults.select,
      props.more != defaults.more,
      props.loading != defaults.loading,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count();
    let signals = (
      [
        self.orientation.clone(),
        self.size.clone(),
        self.align.clone(),
        self.marks.clone(),
      ],
      [
        (self.descriptions.clone(), defaults.descriptions),
        (self.times.clone(), defaults.times),
        (self.long.clone(), defaults.long),
        (self.failed.clone(), defaults.failed),
        (self.current.clone(), defaults.current),
        (self.pending.clone(), defaults.pending),
        (self.upcoming.clone(), defaults.upcoming),
        (self.content.clone(), defaults.content),
        (self.action.clone(), defaults.action),
        (self.select.clone(), defaults.select),
        (self.more.clone(), defaults.more),
        (self.loading.clone(), defaults.loading),
      ],
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Timeline",
        make: Arc::new(make),
        shown: Shown::of(shown),
        fill: !props.orientation.is_vertical(),
        forceable: props.select,
        facts: "4 items".to_owned(),
        event: match (props.select, props.more) {
          (true, true) => "on_select, on_load_more",
          (true, false) => "on_select",
          (false, true) => "on_load_more",
          (false, false) => "",
        },
        code,
        tree,
        controls,
        changed,
        reset: Arc::new(move || {
          for signal in &signals.0 {
            signal.set(0);
          }
          for (signal, value) in &signals.1 {
            signal.set(*value);
          }
        }),
      },
    )
  }
}

impl Component for TimelinePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      orientation: ctx.signal(0),
      size: ctx.signal(0),
      align: ctx.signal(0),
      marks: ctx.signal(0),
      descriptions: ctx.signal(true),
      times: ctx.signal(true),
      long: ctx.signal(false),
      failed: ctx.signal(false),
      current: ctx.signal(true),
      pending: ctx.signal(true),
      upcoming: ctx.signal(true),
      content: ctx.signal(false),
      action: ctx.signal(false),
      select: ctx.signal(false),
      more: ctx.signal(false),
      loading: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let tones = Timeline::new([
      TimelineItem::new("Built")
        .icon("box")
        .variant(TimelineVariant::Success)
        .time("14:02"),
      TimelineItem::new("Tests failed")
        .icon("close")
        .variant(TimelineVariant::Destructive)
        .description("3 of 1,174 failed on macOS.")
        .time("14:09"),
      TimelineItem::new("Retrying").pending(true).current(true).time("14:11"),
      TimelineItem::new("Deploy").upcoming(true),
    ])
    .label("Deploy")
    .style(TimelinePart::Root, |s| s.width(300.0));
    let activity = Timeline::new([
      TimelineItem::new("Olena Koval commented")
        .media(Avatar::new("Olena Koval").size("xs"))
        .time("2 hours ago")
        .child(Card::new().child(Text::new(
          "The blur reads well on the dark theme; on the light one the edge of the panel is lost. Could the \
           border be one step darker there?",
        ))),
      TimelineItem::new("Ivan Zaida pushed 2 commits")
        .media(Avatar::new("Ivan Zaida").size("xs"))
        .description("fix(ui): the border of a frosted panel on the light theme")
        .time("1 hour ago"),
      TimelineItem::new("Taras Bondar requested a review")
        .media(Avatar::new("Taras Bondar").size("xs"))
        .time("just now")
        .action(
          Button::new("Review")
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Sm),
        ),
    ])
    .label("Activity")
    .style(TimelinePart::Root, |s| s.width(420.0));
    let log = Timeline::new([
      TimelineItem::new("Deployed to every region").time("14:21"),
      TimelineItem::new("Smoke tests passed").time("14:17"),
      TimelineItem::new("Canary at 5%")
        .time("14:15")
        .variant(TimelineVariant::Warning),
      TimelineItem::new("Build 0.61.0 signed").time("14:09"),
      TimelineItem::new("Tag v0.61.0 pushed").time("14:02"),
    ])
    .label("Release log")
    .size(TimelineSize::Sm)
    .align(TimelineAlign::TimeColumn)
    .on_load_more(|| {})
    .more_label("Earlier")
    .style(TimelinePart::Root, |s| s.width(360.0));
    let middle = Timeline::new([
      TimelineItem::new("0.60: frosted glass")
        .description("Blur, backdrop filters and the frosted panel.")
        .time("Nov 10"),
      TimelineItem::new("0.59: the design system")
        .description("Recipes, themes and the first forty components.")
        .time("Oct 21"),
      TimelineItem::new("0.58: text")
        .description("Shaping, bidi-free wrapping and the glyph atlas.")
        .time("Sep 30"),
      TimelineItem::new("0.57: the first window")
        .description("wgpu, winit and a tree of elements.")
        .time("Aug 12")
        .upcoming(true),
    ])
    .label("Releases")
    .align(TimelineAlign::Alternate)
    .style(TimelinePart::Root, |s| s.width(560.0));
    let order = Timeline::new([
      TimelineItem::new("Ordered").value(1u32).time("Nov 12, 09:14"),
      TimelineItem::new("Packed").value(2).time("Nov 12, 16:40"),
      TimelineItem::new("Handed to the courier in Lviv")
        .value(3)
        .time("Nov 13, 08:02"),
      TimelineItem::new("Out for delivery")
        .value(4)
        .current(true)
        .pending(true),
      TimelineItem::new("Delivered").value(5).upcoming(true),
    ])
    .label("Order #4021")
    .orientation(Orientation::Horizontal)
    .on_select(|_step: &u32| {});

    let rows = [
      (
        "new",
        "IntoIterator<Item = TimelineItem<T>>",
        "",
        "The items, the first at the top or the start. `item` and `items` add more.",
      ),
      (
        "orientation",
        "Orientation",
        "Vertical",
        "Horizontal: the line across, each item's text under its mark, the items sharing the width and scrolling \
         sideways past their least (160, 120 small).",
      ),
      (
        "size",
        "TimelineSize",
        "Md",
        "Sm: a log, a dot of 8, the time beside the title, items as close as rows of the theme's density.",
      ),
      (
        "align",
        "TimelineAlign",
        "Start",
        "Alternate: either side of the line, falling back to Start in a window narrower than the md breakpoint \
         (768). TimeColumn: the time in a column of its own before the line, or over it across.",
      ),
      (
        "on_select",
        "Fn(&T)",
        "",
        "Told the value of the item pressed: every item is then pressed, the items one stop of Tab with the arrows \
         between them.",
      ),
      (
        "on_load_more / loading",
        "Fn() / bool",
        "",
        "An item at the end that shows more, its mark a spinner while the next page loads. more_label and \
         loading_text say its words.",
      ),
      ("label", "&str", "", "What the timeline is called, to tooling."),
      (
        "TimelineItem::new",
        "&str",
        "",
        "What happened: the title, which wraps.",
      ),
      ("description", "&str", "", "The text under the title, which wraps."),
      (
        "time",
        "&str",
        "",
        "When it happened, as the app writes it, under the description.",
      ),
      (
        "icon / media",
        "impl Into<Name> / impl Into<Element>",
        "a dot",
        "An icon in a well, or the app's mark (an avatar), in place of the dot: every rail is then 24.",
      ),
      (
        "variant",
        "TimelineVariant",
        "Neutral",
        "Info, Success, Warning, Destructive: the color of the mark.",
      ),
      (
        "current / upcoming / pending",
        "bool",
        "false",
        "The one that is now, its dot ringed; one to come, its dot hollow; one running, a spinner for its mark.",
      ),
      (
        "child / children",
        "impl Into<Element>",
        "",
        "Content under the time: a card, a comment, a list of files. The line runs along it.",
      ),
      (
        "action",
        "impl Into<Element>",
        "",
        "The one action, at the end of the item beside its title.",
      ),
      (
        "value",
        "T",
        "()",
        "What the timeline's on_select is told when the item is pressed.",
      ),
      (
        "TimelineItem::on_select",
        "Fn()",
        "",
        "The item is pressed, before the timeline's handler. A second handler is called after the first.",
      ),
      ("disabled", "bool", "false", "An item that is pressed takes nothing."),
      (
        "style / rules",
        "TimelinePart",
        "",
        "Root, Item, Rail, Dot, Mark, Icon, Separator, Content, Header, Title, Description, Time, Body, Action, \
         Opposite, More.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Timeline"],
        title: "Timeline",
        description: "What happened, in its order: the releases of a project, the history of a deploy, the \
                      activity of a document.",
        import: Some("use lurq::ui::{Timeline, TimelineItem};"),
        status: Some("Preview"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Timeline and TimelineItem is a control on the right. A timeline is read, not operated: \
           with on_select its items are pressed, and each press is counted.",
          playground,
        ),
        Section::new(
          "History",
          "The board's timeline. A dot of 10 on a line that runs down to the next item, beside the title, the \
           description and the time; the last item has no line, and this one is to come, its dot hollow.",
          self.frame.demo(
            0,
            board(),
            "Timeline::new([TimelineItem::new(title)..])",
            &[
              "Timeline::new([",
              "    TimelineItem::new(\"Released 0.60\")",
              "        .description(\"Blur and frosted glass land in the renderer.\")",
              "        .time(\"Nov 10\"),",
              "    TimelineItem::new(\"Merged node-blur\")",
              "        .description(\"Blurs follow transforms and rounded clips.\")",
              "        .time(\"Nov 8\"),",
              "    TimelineItem::new(\"Opened pull request\")",
              "        .description(\"Blur a node and its subtree as one layer.\")",
              "        .time(\"Nov 3\")",
              "        .upcoming(true),",
              "])",
              ".label(\"History\")",
            ],
          ),
        ),
        Section::new(
          "Marks and tones",
          "An icon in a well, in the color of the item's tone: one that went well, one that failed. The current \
           item is running, a spinner for its mark, and the next is to come.",
          self.frame.demo(
            1,
            tones,
            ".icon(..).variant(TimelineVariant::Destructive)",
            &[
              "Timeline::new([",
              "    TimelineItem::new(\"Built\").icon(\"box\").variant(TimelineVariant::Success).time(\"14:02\"),",
              "    TimelineItem::new(\"Tests failed\")",
              "        .icon(\"close\")",
              "        .variant(TimelineVariant::Destructive)",
              "        .description(\"3 of 1,174 failed on macOS.\")",
              "        .time(\"14:09\"),",
              "    TimelineItem::new(\"Retrying\").pending(true).current(true).time(\"14:11\"),",
              "    TimelineItem::new(\"Deploy\").upcoming(true),",
              "])",
            ],
          ),
        ),
        Section::new(
          "Activity",
          "Who did what: an avatar for the mark, a comment in a card under its item, and an action at the end of an \
           item. The line runs along what is under an item.",
          self.frame.demo(
            2,
            activity,
            ".media(Avatar::new(name)).child(card).action(button)",
            &[
              "Timeline::new([",
              "    TimelineItem::new(\"Olena Koval commented\")",
              "        .media(Avatar::new(\"Olena Koval\").size(\"xs\"))",
              "        .time(\"2 hours ago\")",
              "        .child(Card::new().child(Text::new(\"The blur reads well ..\"))),",
              "    TimelineItem::new(\"Ivan Zaida pushed 2 commits\")",
              "        .media(Avatar::new(\"Ivan Zaida\").size(\"xs\"))",
              "        .description(\"fix(ui): the border of a frosted panel on the light theme\")",
              "        .time(\"1 hour ago\"),",
              "    TimelineItem::new(\"Taras Bondar requested a review\")",
              "        .media(Avatar::new(\"Taras Bondar\").size(\"xs\"))",
              "        .time(\"just now\")",
              "        .action(Button::new(\"Review\").variant(ButtonVariant::Outline).size(ButtonSize::Sm)),",
              "])",
            ],
          ),
        ),
        Section::new(
          "A log",
          "The small size with the times in a column of their own: a dot of 8, a title of 13, items as close as \
           rows, which follow the theme's density. A feed's history is loaded a page at a time: the line runs on \
           to an item that shows more.",
          self.frame.demo(
            3,
            log,
            ".size(TimelineSize::Sm).align(TimelineAlign::TimeColumn)",
            &[
              "Timeline::new([",
              "    TimelineItem::new(\"Deployed to every region\").time(\"14:21\"),",
              "    TimelineItem::new(\"Smoke tests passed\").time(\"14:17\"),",
              "    TimelineItem::new(\"Canary at 5%\").time(\"14:15\").variant(TimelineVariant::Warning),",
              "    ..",
              "])",
              ".size(TimelineSize::Sm)",
              ".align(TimelineAlign::TimeColumn)",
              ".on_load_more(|| /* fetch the next page */)",
              ".more_label(\"Earlier\")",
            ],
          ),
        ),
        Section::new(
          "Down the middle",
          "Alternate sides: the text before and after the line, item by item, the time on the other side. In a \
           window narrower than the theme's md breakpoint, 768, every item's text is after the line.",
          self.frame.demo(
            4,
            middle,
            ".align(TimelineAlign::Alternate)",
            &[
              "Timeline::new([",
              "    TimelineItem::new(\"0.60: frosted glass\")",
              "        .description(\"Blur, backdrop filters and the frosted panel.\")",
              "        .time(\"Nov 10\"),",
              "    TimelineItem::new(\"0.59: the design system\")..,",
              "    ..",
              "])",
              ".align(TimelineAlign::Alternate)",
            ],
          ),
        ),
        Section::new(
          "Across",
          "An order's steps along a line across, each step's text under its mark. The steps share the width and \
           keep their least, 160: in a narrow room the line scrolls sideways. Every step is pressed and told by \
           its value; the steps are one stop of Tab, and the arrows bring the focused one into view.",
          self.frame.demo(
            5,
            order,
            ".orientation(Orientation::Horizontal).on_select(..)",
            &[
              "Timeline::new([",
              "    TimelineItem::new(\"Ordered\").value(1).time(\"Nov 12, 09:14\"),",
              "    TimelineItem::new(\"Packed\").value(2).time(\"Nov 12, 16:40\"),",
              "    ..",
              "    TimelineItem::new(\"Out for delivery\").value(4).current(true).pending(true),",
              "    TimelineItem::new(\"Delivered\").value(5).upcoming(true),",
              "])",
              ".orientation(Orientation::Horizontal)",
              ".on_select(|step: &u32| /* open the step */)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a timeline and its items are given.",
          props_table(&rows, &["new", "TimelineItem::new"], self.frame.room()),
        ),
      ],
    )
  }
}
