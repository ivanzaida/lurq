//! The page of the notification center: its playground, the board's panel, one opened from a bell whose list the
//! page holds, and the empty and the long panel.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::Element,
  ui::{Avatar, Badge, Button, ButtonVariant, Notification, NotificationCenter, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// What the gallery's notifications say: a title, a line, who it is from, when.
const SAMPLES: [(&str, &str, &str, &str); 6] = [
  ("Ivy Chen requested a review", "feat/node-blur", "Ivy Chen", "2m"),
  ("Build passed", "412 tests in 38 seconds", "C I", "1h"),
  ("Mark published 0.60", "Release notes are live", "Mark Rowe", "1d"),
  (
    "Dana commented on Tokens",
    "The radius scale reads well now.",
    "Dana Kim",
    "2d",
  ),
  ("Nightly build failed", "wasm32: 2 errors in gallery", "C I", "3d"),
  ("Omar joined the workspace", "Design systems", "Omar Aziz", "5d"),
];

/// How many notifications the playground lists.
const COUNTS: [(&str, usize); 3] = [("None", 0), ("3", 3), ("12", 12)];

/// The notification `index` of the samples, read or not.
fn sample(index: usize, unread: bool) -> Notification {
  let (title, description, from, time) = SAMPLES[index % SAMPLES.len()];
  Notification::new(title)
    .description(description)
    .media(Avatar::new(from))
    .time(time)
    .unread(unread)
}

/// The bell: a ghost button with the count of what was not read on it.
fn bell(unread: usize) -> Badge {
  Badge::count(unread).label("unread notifications").on(
    Button::new("Notifications")
      .icon(icons::BELL)
      .icon_only(true)
      .variant(ButtonVariant::Ghost),
  )
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(360.0)
    .max_width(FILL)
    .spacing(10.0)
    .overflow_visible()
    .child(shown)
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(24.0)
    .wrap()
    .overflow_visible()
    .align_items(Alignment::Start)
}

pub struct NotificationCenterPage {
  frame: Frame,
  playground: Playground,
  anchor: ElementRef,
  open: Signal<bool>,
  title: Signal<String>,
  /// How many it lists, as an index of [`COUNTS`].
  count: Signal<usize>,
  mark: Signal<bool>,
  clear: Signal<bool>,
  view_all: Signal<bool>,
  dismissable: Signal<bool>,
  inline: Signal<bool>,
  /// The bell of the section: what it is anchored to, whether it is open, and the list the page holds, each with
  /// whether it was read.
  bell: (ElementRef, Signal<bool>),
  listed: Signal<Vec<(usize, bool)>>,
}

impl NotificationCenterPage {
  /// The playground: the bell and its panel as the panel's props make them.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let title = self.title.get();
    let chosen = self.count.get().min(COUNTS.len() - 1);
    let (count_name, count) = COUNTS[chosen];
    let (mark, clear, view_all) = (self.mark.get(), self.clear.get(), self.view_all.get());
    let (dismissable, inline, open) = (self.dismissable.get(), self.inline.get(), self.open.get());
    let unread = count.min(2);
    let plain = {
      let title = title.clone();
      move |open: Signal<bool>, anchor: ElementRef, record: Arc<dyn Fn(String) + Send + Sync>| {
        let mut center = NotificationCenter::new(open, anchor)
          .title(&title)
          .inline(inline)
          .trigger(bell(unread));
        for index in 0..count {
          let (opened, closed) = (record.clone(), record.clone());
          let mut row = sample(index, index < 2).on_click(move |_| opened(format!("on_click: row {index}")));
          if dismissable {
            row = row.on_close(move || closed(format!("on_close: row {index}")));
          }
          center = center.item(row);
        }
        if mark {
          let said = record.clone();
          center = center.on_mark_all_read(move || said("on_mark_all_read".to_owned()));
        }
        if clear {
          let said = record.clone();
          center = center.on_clear(move || said("on_clear".to_owned()));
        }
        if view_all {
          let said = record.clone();
          center = center.on_view_all(move || said("on_view_all".to_owned()));
        }
        let said = record.clone();
        center.on_dismiss(move || said("on_dismiss".to_owned()))
      }
    };
    let record = self.playground.recorder();
    let shown = plain(self.open.clone(), self.anchor.clone(), Arc::new(record)).force(self.playground.forced());

    let mut code = vec![
      "// open: Signal<bool>, bell: ElementRef".to_owned(),
      "NotificationCenter::new(open.clone(), bell.clone())".to_owned(),
      "    .trigger(Badge::count(unread).on(Button::new(\"Notifications\").icon(icons::BELL).icon_only(true)))"
        .to_owned(),
    ];
    if title != "Notifications" {
      code.push(format!("    .title({title:?})"));
    }
    if count > 0 {
      code.push("    .items(notifications.iter().map(row))".to_owned());
    }
    if mark {
      code.push("    .on_mark_all_read(move || mark_all_read())".to_owned());
    }
    if clear {
      code.push("    .on_clear(move || clear())".to_owned());
    }
    if view_all {
      code.push("    .on_view_all(move || go_to_inbox())".to_owned());
    }
    if inline {
      code.push("    .inline(true)".to_owned());
    }

    let mut tree = vec![
      format!(
        "panel         360 wide · radius 12 · shadow lg   {}",
        if inline {
          "inline"
        } else if open {
          "open"
        } else {
          "closed"
        }
      ),
      format!(
        "├─ header     48 · title 15{}{}",
        if mark { " · Mark all read" } else { "" },
        if clear { " · Clear all" } else { "" }
      ),
    ];
    tree.push(match count {
      0 => format!(
        "{} empty      inbox 24 · \"No notifications\"",
        if view_all { "├─" } else { "└─" }
      ),
      _ => format!(
        "{} list       {count} rows of 56 · scrolls past 360",
        if view_all { "├─" } else { "└─" }
      ),
    });
    if view_all {
      tree.push("└─ footer     40 · View all".to_owned());
    }

    let to = (self.title.clone(), self.count.clone(), self.open.clone());
    let flags = [
      self.mark.clone(),
      self.clear.clone(),
      self.view_all.clone(),
      self.dismissable.clone(),
      self.inline.clone(),
    ];
    let choose = self.count.clone();
    let counts: Vec<(&str, usize)> = COUNTS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Notification Center",
        make: Arc::new(move || plain(Signal::new(false), ElementRef::new(), Arc::new(|_| ())).into()),
        shown: Shown::of(shown),
        fill: false,
        forceable: count > 0 && inline,
        facts: format!("{count_name} listed · {unread} unread"),
        event: "on_mark_all_read / on_clear / on_view_all",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop(
            "items",
            "Notification",
            segmented_fill(&counts, &chosen, move |index| choose.set(index)),
          ),
          divider(),
          flag("on_mark_all_read", &self.mark),
          flag("on_clear", &self.clear),
          flag("on_view_all", &self.view_all),
          flag("rows' on_close", &self.dismissable),
          flag("inline", &self.inline),
          flag("open", &self.open),
        ],
        changed: [
          title != "Notifications",
          count != 3,
          !mark,
          clear,
          !view_all,
          dismissable,
          inline,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to.0.set("Notifications".to_owned());
          to.1.set(1);
          to.2.set(false);
          for flag in &flags {
            flag.set(false);
          }
          flags[0].set(true);
          flags[2].set(true);
        }),
      },
    )
  }

  /// The section's bell: the page holds the list and what was read, and does what the panel tells.
  fn live(&self) -> Element {
    let (anchor, open) = &self.bell;
    let listed = self.listed.get();
    let unread = listed.iter().filter(|(_, read)| !read).count();
    let mut center = NotificationCenter::new(open.clone(), anchor.clone()).trigger(bell(unread));
    for (index, read) in listed.iter().copied() {
      let (mark, drop) = (self.listed.clone(), self.listed.clone());
      center = center.item(
        sample(index, !read)
          .on_click(move |_| {
            mark.update(|listed| {
              for (kept, read) in listed.iter_mut() {
                if *kept == index {
                  *read = true;
                }
              }
            })
          })
          .on_close(move || drop.update(|listed| listed.retain(|(kept, _)| *kept != index))),
      );
    }
    let (mark, clear, restore) = (self.listed.clone(), self.listed.clone(), self.listed.clone());
    let center = center
      .on_mark_all_read(move || mark.update(|listed| listed.iter_mut().for_each(|(_, read)| *read = true)))
      .on_clear(move || clear.set(Vec::new()));
    Row::new()
      .width(FILL)
      .spacing(12.0)
      .wrap()
      .align_items(Alignment::Center)
      .overflow_visible()
      .child(center)
      .child(quiet(&format!("{} listed · {unread} unread", listed.len()), 12.0))
      .child(
        Button::new("Restore the list")
          .variant(ButtonVariant::Outline)
          .on_click(move |_| restore.set(vec![(0, false), (1, false), (2, true), (3, true)])),
      )
      .into()
  }
}

impl Component for NotificationCenterPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      anchor: ElementRef::new(),
      open: ctx.signal(false),
      title: ctx.signal("Notifications".to_owned()),
      count: ctx.signal(1),
      mark: ctx.signal(true),
      clear: ctx.signal(false),
      view_all: ctx.signal(true),
      dismissable: ctx.signal(false),
      inline: ctx.signal(false),
      bell: (ElementRef::new(), ctx.signal(false)),
      listed: ctx.signal(vec![(0, false), (1, false), (2, true), (3, true)]),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let panel = || NotificationCenter::new(Signal::new(true), ElementRef::new()).inline(true);
    let board = items().child(item(
      panel()
        .items((0..3).map(|index| sample(index, index < 2).on_click(|_| ())))
        .on_mark_all_read(|| ())
        .on_view_all(|| ()),
      "notification center",
    ));
    let live = self.live();
    let ends = items()
      .child(item(panel().on_mark_all_read(|| ()), "empty"))
      .child(item(
        panel()
          .items((0..12).map(|index| sample(index, index < 2).on_click(|_| ())))
          .on_clear(|| ())
          .max_height(224.0),
        "12 rows · max_height(224.0)",
      ));
    let table = [
      (
        "new",
        "(Signal<bool>, impl Into<ElementRef>)",
        "",
        "Whether it is open, and the bell it is shown beside. Both are the app's, kept from one render to the next.",
      ),
      (
        "trigger",
        "impl Into<Element>",
        "",
        "What opens and closes it: the bell, a Button or a Badge with its count on one. It is the anchor.",
      ),
      (
        "title",
        "&str",
        "Notifications",
        "What it is called, at its top and to tooling.",
      ),
      (
        "item / items",
        "Notification",
        "",
        "What it lists, each drawn as a row whatever its own variant.",
      ),
      (
        "on_mark_all_read",
        "impl Fn()",
        "",
        "Draws \"Mark all read\" in the header and is called on a press of it. It takes no press while nothing is \
         unread.",
      ),
      (
        "on_clear",
        "impl Fn()",
        "",
        "Draws \"Clear all\" in the header and is called on a press of it. It takes no press while the list is \
         empty.",
      ),
      (
        "on_view_all",
        "impl Fn()",
        "",
        "Draws the footer, \"View all\", and is called on a press of it, after the panel closed.",
      ),
      (
        "mark_all_read_label, clear_label, view_all_label",
        "&str",
        "",
        "What each of its controls says.",
      ),
      (
        "empty_text",
        "&str",
        "No notifications",
        "What it says under an inbox while it lists nothing.",
      ),
      (
        "empty",
        "impl Into<Element>",
        "",
        "The app's own empty state in place of the inbox and the words.",
      ),
      (
        "on_dismiss",
        "impl Fn()",
        "",
        "Told after Escape or a press outside closed it; not when the app sets the signal.",
      ),
      (
        "dismiss_on_escape, dismiss_on_outside_press",
        "bool",
        "true",
        "Whether each closes it.",
      ),
      (
        "placement",
        "Placement",
        "BottomEnd",
        "The side of the bell it is on; the other where there is no room.",
      ),
      (
        "offset",
        "impl Into<SpacingValue>",
        "Xs",
        "How far it is from the bell.",
      ),
      (
        "collision",
        "CollisionStrategy",
        "FlipThenClamp",
        "What it does where there is no room on its side.",
      ),
      ("width", "impl Into<Length>", "360", "How wide the panel is."),
      (
        "max_height",
        "impl Into<Length>",
        "360",
        "How high the list is at most before it scrolls.",
      ),
      (
        "inline",
        "bool",
        "false",
        "Draws the panel where it is declared, open or not: a panel of a page.",
      ),
      ("label", "&str", "the title", "What it is called to tooling."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces & overlays", "Notification Center"],
        title: "Notification Center",
        description: "The panel that lists an app's notifications, opened from its bell. The list, what was read and \
                      what was dismissed are the app's: the panel tells what a person did, and is given the list as \
                      it is then.",
        import: Some("use lurq::ui::{Notification, NotificationCenter};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The bell opens the panel; inline draws it on the stage. Every press of a control of the panel and of a \
           row is counted.",
          playground,
        ),
        Section::new(
          "Notification center",
          "The board's panel: the header with what marks all read, three rows with a line between two, and the \
           footer that leads to all of them.",
          self.frame.demo(
            0,
            board,
            "NotificationCenter::new(open, bell).items(..).on_mark_all_read(..).on_view_all(..)",
            &[
              "NotificationCenter::new(open.clone(), bell.clone())",
              "    .item(",
              "        Notification::new(\"Ivy Chen requested a review\")",
              "            .description(\"feat/node-blur\")",
              "            .media(Avatar::new(\"Ivy Chen\"))",
              "            .time(\"2m\")",
              "            .unread(true)",
              "            .on_click(move |_| open_review()),",
              "    )",
              "    .on_mark_all_read(move || mark_all_read())",
              "    .on_view_all(move || go_to_inbox())",
              "    .inline(true)",
            ],
          ),
        ),
        Section::new(
          "From a bell",
          "The bell has the count of what was not read. The page holds the list: a press of a row reads it, its \
           close control takes it away, and the header marks all read or clears.",
          self.frame.demo(
            1,
            live,
            ".trigger(Badge::count(unread).on(Button::new(\"Notifications\").icon(icons::BELL).icon_only(true)))",
            &[
              "// open: Signal<bool>, bell: ElementRef, listed: Signal<Vec<(Id, bool)>>",
              "let unread = listed.get().iter().filter(|(_, read)| !read).count();",
              "NotificationCenter::new(open.clone(), bell.clone())",
              "    .trigger(Badge::count(unread).label(\"unread notifications\").on(",
              "        Button::new(\"Notifications\").icon(icons::BELL).icon_only(true).variant(ButtonVariant::Ghost),",
              "    ))",
              "    .items(listed.get().into_iter().map(|(id, read)| row(id, read)))",
              "    .on_mark_all_read(move || listed.update(|all| all.iter_mut().for_each(|(_, read)| *read = true)))",
              "    .on_clear(move || listed.set(Vec::new()))",
            ],
          ),
        ),
        Section::new(
          "Empty and long",
          "With nothing to list it says so, and its controls take no press. A list higher than the panel lets it be \
           scrolls under the header.",
          self.frame.demo(
            2,
            ends,
            ".empty_text(..) · .max_height(..)",
            &[
              "NotificationCenter::new(open.clone(), bell.clone()).on_mark_all_read(|| ())",
              "",
              "NotificationCenter::new(open.clone(), bell.clone())",
              "    .items(notifications)",
              "    .on_clear(move || clear())",
              "    .max_height(224.0)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a notification center is given.",
          props_table(&table, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
