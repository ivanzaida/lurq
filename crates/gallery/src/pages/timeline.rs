//! The page of the timeline: its playground, the board's history of a project, a deploy with its marks and tones, and
//! the activity of a document with avatars and content under its items.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::Text,
  core::Signal,
  node::Element,
  ui::{Avatar, Button, ButtonSize, ButtonVariant, Card, Timeline, TimelineItem, TimelinePart, TimelineVariant},
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
}

pub struct TimelinePage {
  frame: Frame,
  playground: Playground,
  /// Which of [`MARKS`] stands for the marks.
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
}

/// The playground's timeline: a deploy, as `props` make it, its items telling `record` they were pressed.
fn deploy(props: Props, record: Option<Arc<dyn Fn(String) + Send + Sync>>) -> Timeline {
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
      let mut item = TimelineItem::new(title);
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
      if props.select
        && let Some(record) = &record
      {
        let (record, title) = (record.clone(), title.to_string());
        item = item.on_select(move || record(format!("on_select(\"{title}\")")));
      }
      item
    });
  Timeline::new(items)
    .label("Deploy")
    .style(TimelinePart::Root, |s| s.width(340.0))
}

impl TimelinePage {
  fn props(&self) -> Props {
    Props {
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
    let mut first = format!("    TimelineItem::new(\"Built\"){mark}");
    if props.descriptions {
      first.push_str(".description(\"Compiled in release mode with LTO.\")");
    }
    if props.times {
      first.push_str(".time(\"14:02\")");
    }
    if props.content {
      first.push_str(".child(Card::new().description(\"lurq-0.61.0-..zip, 18.4 MB\"))");
    }
    if props.select {
      first.push_str(".on_select(|| /* open it */)");
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

    let mut controls = vec![
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
    ];
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
      flag("on_select", &self.select),
    ]);
    let defaults = Props {
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
    };
    let changed = [
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
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count();
    let signals = (
      self.marks.clone(),
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
      ],
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Timeline",
        make: Arc::new(make),
        shown: Shown::of(shown),
        fill: false,
        forceable: props.select,
        facts: "4 items".to_owned(),
        event: if props.select { "on_select" } else { "" },
        code,
        tree,
        controls,
        changed,
        reset: Arc::new(move || {
          signals.0.set(0);
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

    let rows = [
      (
        "new",
        "IntoIterator<Item = TimelineItem>",
        "",
        "The items, the first at the top. `item` and `items` add more.",
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
        "on_select",
        "Fn()",
        "",
        "The item is pressed and a stop of Tab. A second handler is called after the first.",
      ),
      ("disabled", "bool", "false", "An item that is pressed takes nothing."),
      (
        "style / rules",
        "TimelinePart",
        "",
        "Root, Item, Rail, Dot, Mark, Icon, Separator, Content, Title, Action, Description, Time, Body.",
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
          "Props",
          "What a timeline and its items are given.",
          props_table(&rows, &["new", "TimelineItem::new"], self.frame.room()),
        ),
      ],
    )
  }
}
