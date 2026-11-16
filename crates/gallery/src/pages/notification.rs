//! The page of the notification: its playground, the board's card, read and unread with a time, rows of a list,
//! and a notification pressed as a whole.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Avatar, Notification, NotificationVariant, Progress, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

const TITLE: &str = "Review requested";
const SAYS: &str = "Ivy asked you to review feat/node-blur.";

/// The variants, as the code names them.
const VARIANTS: [(NotificationVariant, &str); 2] =
  [(NotificationVariant::Card, "Card"), (NotificationVariant::Row, "Row")];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(340.0)
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

pub struct NotificationPage {
  frame: Frame,
  playground: Playground,
  title: Signal<String>,
  description: Signal<String>,
  time: Signal<String>,
  variant: Signal<usize>,
  unread: Signal<bool>,
  media: Signal<bool>,
  button: Signal<bool>,
  closable: Signal<bool>,
  pressable: Signal<bool>,
  body: Signal<bool>,
  /// The notifications of the last section that were not dismissed, and those that were read.
  listed: Signal<Vec<usize>>,
  read: Signal<Vec<usize>>,
}

impl NotificationPage {
  /// The playground: the notification as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, description, time) = (self.title.get(), self.description.get(), self.time.get());
    let (variant, variant_name) = VARIANTS[self.variant.get().min(VARIANTS.len() - 1)];
    let (unread, media, button) = (self.unread.get(), self.media.get(), self.button.get());
    let (closable, pressable, body) = (self.closable.get(), self.pressable.get(), self.body.get());
    let row = variant == NotificationVariant::Row;
    let plain = {
      let (title, description, time) = (title.clone(), description.clone(), time.clone());
      move |record: Arc<dyn Fn(String) + Send + Sync>| {
        let mut notification = Notification::new(&title).variant(variant).unread(unread);
        if !description.is_empty() {
          notification = notification.description(&description);
        }
        if !time.is_empty() {
          notification = notification.time(&time);
        }
        if media {
          notification = notification.media(Avatar::new("Ivy Chen"));
        }
        if button {
          let said = record.clone();
          notification = notification.button("Review", move || said("pressed: Review".to_owned()));
        }
        if body {
          notification = notification.child(Progress::new(0.6).label("Checks"));
        }
        if closable {
          let said = record.clone();
          notification = notification.on_close(move || said("on_close".to_owned()));
        }
        if pressable {
          let said = record.clone();
          notification = notification.on_click(move |_| said("on_click".to_owned()));
        }
        notification
      }
    };
    let record = self.playground.recorder();
    let shown = plain(Arc::new(record)).force(self.playground.forced());
    let make = {
      let plain = plain.clone();
      move || Column::new().width(340.0).child(plain(Arc::new(|_| ()))).into()
    };

    let mut code = vec![format!("Notification::new({title:?})")];
    if !description.is_empty() {
      code.push(format!("    .description({description:?})"));
    }
    if media {
      code.push("    .media(Avatar::new(\"Ivy Chen\"))".to_owned());
    }
    if !time.is_empty() {
      code.push(format!("    .time({time:?})"));
    }
    if unread {
      code.push("    .unread(true)".to_owned());
    }
    if row {
      code.push("    .variant(NotificationVariant::Row)".to_owned());
    }
    if button {
      code.push("    .button(\"Review\", || open_review())".to_owned());
    }
    if body {
      code.push("    .child(Progress::new(0.6).label(\"Checks\"))".to_owned());
    }
    if closable {
      code.push("    .on_close(move || dismiss(id))".to_owned());
    }
    if pressable {
      code.push("    .on_click(move |_| open(id))".to_owned());
    }

    let mut tree = vec![format!(
      "root          {}   {}",
      self.playground.measured(),
      if row {
        "a row · 12 and 16 from its sides"
      } else {
        "a card · radius 8 · padded 16 · shadow md"
      }
    )];
    if row {
      tree.push(format!(
        "├─ unread     7 · {}",
        if unread { "info" } else { "its room kept" }
      ));
    }
    tree.push(format!(
      "├─ media      {}",
      if media { "the avatar, 32" } else { "the bell, 16" }
    ));
    tree.push(format!(
      "{} content    title{}{}{}{}",
      if row && (!time.is_empty() || closable) {
        "├─"
      } else {
        "└─"
      },
      if row || time.is_empty() { "" } else { " · time" },
      if !row && unread { " · dot" } else { "" },
      if description.is_empty() { "" } else { " · description" },
      if button || (closable && !row) {
        " · actions"
      } else {
        ""
      }
    ));
    if row && !time.is_empty() {
      tree.push(format!(
        "{} time       mono 11 · muted",
        if closable { "├─" } else { "└─" }
      ));
    }
    if row && closable {
      tree.push("└─ close      16 · muted".to_owned());
    }

    let to = (
      self.title.clone(),
      self.description.clone(),
      self.time.clone(),
      self.variant.clone(),
    );
    let flags = [
      self.unread.clone(),
      self.media.clone(),
      self.button.clone(),
      self.closable.clone(),
      self.pressable.clone(),
      self.body.clone(),
    ];
    let choose = self.variant.clone();
    let variants: Vec<(&str, usize)> = VARIANTS
      .iter()
      .enumerate()
      .map(|(index, (_, name))| (*name, index))
      .collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Notification",
        make: Arc::new(make),
        shown: Shown::built(move |_| {
          Column::new()
            .width(340.0)
            .max_width(FILL)
            .overflow_visible()
            .child(shown)
            .into()
        }),
        fill: false,
        forceable: pressable || (row && closable),
        facts: format!(
          "{} · {}",
          variant_name.to_lowercase(),
          if unread { "unread" } else { "read" }
        ),
        event: "on_click",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          prop("time", "&str", field(&self.time)),
          prop(
            "variant",
            "NotificationVariant",
            segmented_fill(&variants, &self.variant.get(), move |index| choose.set(index)),
          ),
          divider(),
          flag("unread", &self.unread),
          flag("media", &self.media),
          flag("button", &self.button),
          flag("on_close", &self.closable),
          flag("on_click", &self.pressable),
          flag("child", &self.body),
        ],
        changed: [
          title != TITLE,
          description != SAYS,
          !time.is_empty(),
          row,
          unread,
          media,
          !button,
          !closable,
          pressable,
          body,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to.0.set(TITLE.to_owned());
          to.1.set(SAYS.to_owned());
          to.2.set(String::new());
          to.3.set(0);
          for flag in &flags {
            flag.set(false);
          }
          flags[2].set(true);
          flags[3].set(true);
        }),
      },
    )
  }

  /// The rows of the last section: what the page lists, each read on a press and taken away on its close control.
  fn rows(&self) -> Element {
    let all = [
      ("Ivy Chen requested a review", "feat/node-blur", "Ivy Chen", "2m"),
      ("Build passed", "412 tests in 38 seconds", "C I", "1h"),
      ("Mark published 0.60", "Release notes are live", "Mark Rowe", "1d"),
    ];
    let (listed, read) = (self.listed.get(), self.read.get());
    let mut list = Column::new().width(360.0).max_width(FILL);
    for (index, (title, description, from, time)) in all.into_iter().enumerate() {
      if !listed.contains(&index) {
        continue;
      }
      let (mark, drop) = (self.read.clone(), self.listed.clone());
      list = list.child(
        Notification::new(title)
          .description(description)
          .media(Avatar::new(from))
          .time(time)
          .unread(!read.contains(&index))
          .variant(NotificationVariant::Row)
          .on_click(move |_| {
            mark.update(|read| {
              if !read.contains(&index) {
                read.push(index);
              }
            })
          })
          .on_close(move || drop.update(|listed| listed.retain(|kept| *kept != index))),
      );
    }
    if listed.is_empty() {
      let (listed, read) = (self.listed.clone(), self.read.clone());
      return lurq::ui::Button::new("Show them again")
        .variant(lurq::ui::ButtonVariant::Outline)
        .on_click(move |_| {
          listed.set(vec![0, 1, 2]);
          read.set(vec![2]);
        })
        .into();
    }
    list.into()
  }
}

impl Component for NotificationPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      title: ctx.signal(TITLE.to_owned()),
      description: ctx.signal(SAYS.to_owned()),
      time: ctx.signal(String::new()),
      variant: ctx.signal(0),
      unread: ctx.signal(false),
      media: ctx.signal(false),
      button: ctx.signal(true),
      closable: ctx.signal(true),
      pressable: ctx.signal(false),
      body: ctx.signal(false),
      listed: ctx.signal(vec![0, 1, 2]),
      read: ctx.signal(vec![2]),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = items().child(item(
      Notification::new(TITLE)
        .description(SAYS)
        .button("Review", || ())
        .on_close(|| ()),
      "notification",
    ));
    let states = items()
      .child(item(
        Notification::new("Build passed")
          .description("412 tests in 38 seconds")
          .icon(icons::CHECK_CIRCLE)
          .time("1h")
          .unread(true),
        "unread · time · icon",
      ))
      .child(item(
        Notification::new("Ivy Chen mentioned you")
          .description("Can you look at the blur on a rounded node before the release?")
          .media(Avatar::new("Ivy Chen"))
          .time("2m")
          .button("Reply", || ())
          .button("Mute", || ()),
        "media · two buttons",
      ))
      .child(item(Notification::new("Title alone").icon(None), "title alone"));
    let rows = items().child(
      Column::new()
        .width(360.0)
        .max_width(FILL)
        .spacing(10.0)
        .child(self.rows())
        .child(quiet("rows · on_click reads · on_close dismisses", 12.0)),
    );
    let table = [
      (
        "new",
        "&str",
        "",
        "The title: what happened, in a few words. It is what the notification is called.",
      ),
      ("description", "&str", "", "More of it under the title, muted."),
      (
        "media",
        "impl Into<Element>",
        "",
        "Who or what it is from, before the text in place of the icon: an Avatar, an app's mark.",
      ),
      (
        "icon",
        "impl Into<IconChoice>",
        "bell",
        "An icon of the theme's set in place of the bell, or none.",
      ),
      (
        "time",
        "&str",
        "",
        "When it happened, as the app words it: mono 11, muted.",
      ),
      (
        "unread",
        "bool",
        "false",
        "Whether it was not read: the dot in the info color, and a row's title medium.",
      ),
      (
        "button",
        "(&str, impl Fn())",
        "",
        "Adds a button under the text: the first is the primary one, the others ghosts.",
      ),
      (
        "action",
        "impl Into<Element>",
        "",
        "Adds something of the app's own to the actions. It adds: a notification has several.",
      ),
      (
        "child / children",
        "impl Into<Element>",
        "",
        "Elements under the description, for more than text.",
      ),
      (
        "variant",
        "NotificationVariant",
        "Card",
        "Card, with an edge and a shadow, or Row, as a list of them draws each.",
      ),
      (
        "on_click",
        "impl IntoMouseEventHandler",
        "",
        "A press of the whole: it opens what it is about. The notification is a button and a stop of Tab, its own \
         buttons pressed apart from it.",
      ),
      (
        "on_close",
        "impl Fn()",
        "",
        "Draws what dismisses it, a Dismiss button on a card and a close control on a row, and is called on a press \
         of it. The app takes the notification out of what it lists.",
      ),
      (
        "close_label",
        "&str",
        "Dismiss",
        "What dismisses it says and is called.",
      ),
      ("unread_label", "&str", "Unread", "What its dot is called to tooling."),
      ("label", "&str", "the title", "What it is called to tooling."),
      (
        "force",
        "Option<Interaction>",
        "None",
        "Draws it, where it is pressed, hovered, pressed or focused with nothing on it.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces & overlays", "Notification"],
        title: "Notification",
        description: "Something that happened which a person may act on now or later: who it is from, a title and a \
                      line, when, what can be done about it, whether it was read. It stays where the app lists it \
                      until it is dismissed; a toast leaves by itself, and an alert is part of its page.",
        import: Some("use lurq::ui::{Notification, NotificationVariant};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Notification is a control on the right. With on_click on, the whole notification is pressed, \
           and its buttons are pressed apart from it; each press is counted.",
          playground,
        ),
        Section::new(
          "Notification",
          "The board's card: the bell, a title over a line, the primary button, and what dismisses it as a ghost \
           beside it.",
          self.frame.demo(
            0,
            board,
            "Notification::new(..).description(..).button(..).on_close(..)",
            &[
              "Notification::new(\"Review requested\")",
              "    .description(\"Ivy asked you to review feat/node-blur.\")",
              "    .button(\"Review\", || open_review())",
              "    .on_close(move || dismiss(id))",
            ],
          ),
        ),
        Section::new(
          "Unread, time, media",
          "One that was not read has a dot after its time. Who it is from stands in place of the bell, and a second \
           button is a ghost. A title alone is enough.",
          self.frame.demo(
            1,
            states,
            ".unread(true).time(..).media(..)",
            &[
              "Notification::new(\"Build passed\")",
              "    .description(\"412 tests in 38 seconds\")",
              "    .icon(icons::CHECK_CIRCLE)",
              "    .time(\"1h\")",
              "    .unread(true)",
              "",
              "Notification::new(\"Ivy Chen mentioned you\")",
              "    .description(\"Can you look at the blur on a rounded node before the release?\")",
              "    .media(Avatar::new(\"Ivy Chen\"))",
              "    .time(\"2m\")",
              "    .button(\"Reply\", || reply())",
              "    .button(\"Mute\", || mute())",
              "",
              "Notification::new(\"Title alone\").icon(None)",
            ],
          ),
        ),
        Section::new(
          "Rows",
          "As a list draws them, and as a notification center does: the dot, who it is from, the text and the time. \
           The page holds the list: a press of a row reads it, and its close control takes it away.",
          self.frame.demo(
            2,
            rows,
            ".variant(NotificationVariant::Row).on_click(..).on_close(..)",
            &[
              "// read: Signal<Vec<Id>>, listed: Signal<Vec<Id>>",
              "Notification::new(\"Ivy Chen requested a review\")",
              "    .description(\"feat/node-blur\")",
              "    .media(Avatar::new(\"Ivy Chen\"))",
              "    .time(\"2m\")",
              "    .unread(!read.get().contains(&id))",
              "    .variant(NotificationVariant::Row)",
              "    .on_click(move |_| read.update(|read| read.push(id)))",
              "    .on_close(move || listed.update(|listed| listed.retain(|kept| *kept != id)))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a notification is given. Its title is all it needs.",
          props_table(&table, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
