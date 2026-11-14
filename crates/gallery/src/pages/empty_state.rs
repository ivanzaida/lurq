//! The page of the empty state: its playground, the three sizes the board draws, an element of the app's in place
//! of the icon, and an empty state centred in the room of a panel.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{BorderSize, PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::{Element, border::Border},
  ui::{Avatar, AvatarGroup, Button, ButtonSize, ButtonVariant, EmptyState, EmptyStateSize, Length, Skeleton, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The sizes, as the code names them.
const SIZES: [(EmptyStateSize, &str, &str); 3] = [
  (EmptyStateSize::Sm, "sm", "EmptyStateSize::Sm"),
  (EmptyStateSize::Md, "md", "EmptyStateSize::Md"),
  (EmptyStateSize::Lg, "lg", "EmptyStateSize::Lg"),
];

/// The icons the playground offers, as the code names them: none first.
const ICONS: [(&str, Option<&str>, &str); 5] = [
  ("none", None, ""),
  ("inbox", Some(icons::INBOX), "icons::INBOX"),
  ("search-close", Some(icons::SEARCH_CLOSE), "icons::SEARCH_CLOSE"),
  ("window", Some(icons::WINDOW), "icons::WINDOW"),
  ("folder", Some(icons::FOLDER), "icons::FOLDER"),
];

/// How many actions the playground gives it.
const ACTIONS: [(&str, usize); 3] = [("none", 0), ("one", 1), ("two", 2)];

/// Whether it has an edge: as its size has it, or as told.
const EDGES: [(&str, Option<bool>); 3] = [("by size", None), ("edge", Some(true)), ("none", Some(false))];

/// Something on a stage with what it is under it.
fn item(width: f32, shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(width)
    .spacing(9.0)
    .child(shown)
    .child(quiet(caption, 12.0))
}

/// The buttons of an empty state of `size`: as large as the board draws them there.
fn button(label: &str, size: EmptyStateSize, variant: ButtonVariant) -> Button {
  let button = Button::new(label).variant(variant);
  match size {
    EmptyStateSize::Sm => button.size(ButtonSize::Sm),
    EmptyStateSize::Md => button,
    EmptyStateSize::Lg => button.size(ButtonSize::Lg),
  }
}

pub struct EmptyStatePage {
  frame: Frame,
  playground: Playground,
  title: Signal<String>,
  description: Signal<String>,
  icon: Signal<usize>,
  size: Signal<usize>,
  actions: Signal<usize>,
  edge: Signal<usize>,
  media: Signal<bool>,
}

impl EmptyStatePage {
  /// The playground: the empty state as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, description) = (self.title.get(), self.description.get());
    let (_, icon, icon_code) = ICONS[self.icon.get().min(ICONS.len() - 1)];
    let (size, size_name, size_code) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let (_, actions) = ACTIONS[self.actions.get().min(ACTIONS.len() - 1)];
    let (_, edge) = EDGES[self.edge.get().min(EDGES.len() - 1)];
    let media = self.media.get();

    // A press of an action is the event of an empty state: the buttons are the app's, and so is what they do.
    let record = self.playground.recorder();
    let pressed = move |label: &'static str| {
      let record = record.clone();
      move |_| record(format!("on_click({label:?})"))
    };
    let plain = {
      let (title, description) = (title.clone(), description.clone());
      move || -> Element {
        let mut empty = EmptyState::new(&title).size(size);
        if !description.is_empty() {
          empty = empty.description(&description);
        }
        if let Some(icon) = icon {
          empty = empty.icon(icon);
        }
        if media {
          empty = empty.media(AvatarGroup::new([
            Avatar::new("Ada Lovelace"),
            Avatar::new("Alan Turing"),
          ]));
        }
        if actions == 2 {
          empty = empty.action(button("Learn more", size, ButtonVariant::Ghost).on_click(pressed("Learn more")));
        }
        if actions >= 1 {
          empty = empty.action(
            button("New window", size, ButtonVariant::Outline)
              .icon(icons::PLUS)
              .on_click(pressed("New window")),
          );
        }
        if let Some(edge) = edge {
          empty = empty.bordered(edge);
        }
        empty.into()
      }
    };

    let mut code = vec![format!("EmptyState::new({title:?})")];
    if !description.is_empty() {
      code.push(format!("    .description({description:?})"));
    }
    if icon.is_some() {
      code.push(format!("    .icon({icon_code})"));
    }
    if media {
      code
        .push("    .media(AvatarGroup::new([Avatar::new(\"Ada Lovelace\"), Avatar::new(\"Alan Turing\")]))".to_owned());
    }
    if size != EmptyStateSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    if let Some(edge) = edge {
      code.push(format!("    .bordered({edge})"));
    }
    let sized = match size {
      EmptyStateSize::Sm => ".size(ButtonSize::Sm)",
      EmptyStateSize::Md => "",
      EmptyStateSize::Lg => ".size(ButtonSize::Lg)",
    };
    if actions == 2 {
      code.push(format!(
        "    .action(Button::new(\"Learn more\").variant(ButtonVariant::Ghost){sized})"
      ));
    }
    if actions >= 1 {
      code.push(format!(
        "    .action(Button::new(\"New window\").variant(ButtonVariant::Outline){sized}.icon(icons::PLUS))"
      ));
    }

    let bordered = edge.unwrap_or(size == EmptyStateSize::Md);
    let mut tree = vec![format!(
      "root      {}   {size_name}{}   status",
      self.playground.measured(),
      if bordered { " · edge" } else { "" }
    )];
    if media {
      tree.push("├─ the app's element".to_owned());
    } else if icon.is_some() {
      tree.push("├─ media   the well".to_owned());
      tree.push("│  └─ icon".to_owned());
    }
    tree.push("├─ text".to_owned());
    tree.push("│  ├─ title".to_owned());
    if !description.is_empty() {
      tree.push("│  └─ description".to_owned());
    }
    if actions > 0 {
      tree.push(format!("└─ actions   {actions}"));
    }

    let glyphs: Vec<(&str, usize)> = ICONS.iter().enumerate().map(|(index, i)| (i.0, index)).collect();
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, s)| (s.1, index)).collect();
    let counts: Vec<(&str, usize)> = ACTIONS.iter().enumerate().map(|(index, a)| (a.0, index)).collect();
    let edges: Vec<(&str, usize)> = EDGES.iter().enumerate().map(|(index, e)| (e.0, index)).collect();
    let (to_icon, to_size, to_actions, to_edge) = (
      self.icon.clone(),
      self.size.clone(),
      self.actions.clone(),
      self.edge.clone(),
    );
    let reset = (
      self.title.clone(),
      self.description.clone(),
      self.icon.clone(),
      self.size.clone(),
      self.actions.clone(),
      self.edge.clone(),
      self.media.clone(),
    );
    let make = plain.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "EmptyState",
        make: Arc::new(move || make()),
        shown: plain(),
        fill: true,
        forceable: false,
        facts: size_name.to_owned(),
        event: if actions == 0 { "" } else { "on_click" },
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          prop(
            "icon",
            "impl Into<Name>",
            self
              .playground
              .select(0, &glyphs, &self.icon.get(), move |index| to_icon.set(index)),
          ),
          flag("media", &self.media),
          divider(),
          prop(
            "size",
            "EmptyStateSize",
            segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
          ),
          prop(
            "bordered",
            "bool",
            segmented_fill(&edges, &self.edge.get(), move |index| to_edge.set(index)),
          ),
          prop(
            "action",
            "impl Into<Element>",
            segmented_fill(&counts, &self.actions.get(), move |index| to_actions.set(index)),
          ),
        ],
        changed: [
          title != "No windows yet",
          description != "Open a window to see it listed here.",
          self.icon.get() != 1,
          media,
          size != EmptyStateSize::Md,
          edge.is_some(),
          actions != 1,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          reset.0.set("No windows yet".to_owned());
          reset.1.set("Open a window to see it listed here.".to_owned());
          reset.2.set(1);
          reset.3.set(1);
          reset.4.set(1);
          reset.5.set(0);
          reset.6.set(false);
        }),
      },
    )
  }
}

impl Component for EmptyStatePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      title: ctx.signal("No windows yet".to_owned()),
      description: ctx.signal("Open a window to see it listed here.".to_owned()),
      icon: ctx.signal(1),
      size: ctx.signal(1),
      actions: ctx.signal(1),
      edge: ctx.signal(0),
      media: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let sizes = Row::new()
      .width(FILL)
      .spacing(28.0)
      .wrap()
      .align_items(Alignment::Start)
      .child(item(
        320.0,
        EmptyState::new("No windows yet")
          .description("Open a window to see it listed here.")
          .icon(icons::INBOX)
          .action(
            Button::new("New window")
              .variant(ButtonVariant::Outline)
              .icon(icons::PLUS),
          ),
        "md · in an edge",
      ))
      .child(item(
        320.0,
        EmptyState::new("No crates match “blurr”")
          .description("Check the spelling or clear the filter.")
          .icon(icons::SEARCH_CLOSE)
          .size(EmptyStateSize::Sm)
          .action(
            Button::new("Clear filter")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm),
          ),
        "sm · in a table",
      ))
      .child(item(
        328.0,
        EmptyState::new("You're all caught up")
          .description("New review requests will show up here.")
          .icon(icons::INBOX)
          .size(EmptyStateSize::Lg)
          .action(
            Button::new("Refresh")
              .variant(ButtonVariant::Secondary)
              .size(ButtonSize::Lg),
          ),
        "lg · on a phone",
      ));
    let media = Row::new().width(FILL).child(item(
      320.0,
      EmptyState::new("No one here yet")
        .description("Invite people to work on this project with you.")
        .media(AvatarGroup::new([
          Avatar::new("Ada Lovelace"),
          Avatar::new("Alan Turing"),
          Avatar::new("Grace Hopper"),
        ]))
        .action(Button::new("Not now").variant(ButtonVariant::Ghost))
        .action(Button::new("Invite").icon(icons::PLUS)),
      "an element of the app's, two actions",
    ));
    // A panel with a list that has nothing in it: the empty state takes the room the list would.
    let panel = Column::new()
      .width(FILL)
      .height(280.0)
      .rounded(RadiusSize::Lg)
      .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
      .child(
        Row::new()
          .width(FILL)
          .padding(12.0)
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(Skeleton::line().width(120.0))
          .child(Row::new().flex(1.0))
          .child(Skeleton::line().width(60.0)),
      )
      .child(
        Column::new().width(FILL).flex(1.0).child(
          EmptyState::new("No messages")
            .description("Messages you are sent show up here.")
            .icon(icons::INBOX)
            .size(EmptyStateSize::Sm)
            .height(Length::Fill),
        ),
      );
    let rows = [
      (
        "title",
        "&str",
        "",
        "What is empty, in a line. It is what the empty state is called.",
      ),
      ("description", "&str", "", "Why, or what to do, under the title."),
      (
        "icon",
        "impl Into<Name>",
        "",
        "An icon of the theme's set over the title, in its well.",
      ),
      (
        "media",
        "impl Into<Element>",
        "",
        "An element of the app's in place of the icon and its well.",
      ),
      (
        "action",
        "impl Into<Element>",
        "",
        "Something to do under the text; a second one after it. The main one last.",
      ),
      (
        "size",
        "EmptyStateSize",
        "Md",
        "Sm: the icon alone, everything 10 apart. Md: a well of 40. Lg: a round well of 64, a title of 18.",
      ),
      (
        "bordered",
        "bool",
        "by size",
        "An edge around it: the medium size has one, the small and the large none.",
      ),
      (
        "height",
        "impl Into<Length>",
        "",
        "A height of its own, in which what it holds is centred.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Empty State"],
        title: "Empty State",
        description: "What stands where content would be when there is none, and what to do about it: no results, \
                      no windows yet, all caught up.",
        import: Some("use lurq::ui::EmptyState;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of EmptyState is a control on the right. It is read; what it gives to do is operated as itself.",
          playground,
        ),
        Section::new(
          "Sizes",
          "The medium one in an edge, with its icon in a well of 40; the small one as a table draws it, the icon alone \
           and everything 10 apart; the large one as a phone draws it. The buttons are the app's, as large as the \
           place has them.",
          self.frame.demo(
            0,
            sizes,
            ".size(EmptyStateSize::Sm)",
            &[
              "EmptyState::new(\"No windows yet\")",
              "    .description(\"Open a window to see it listed here.\")",
              "    .icon(icons::INBOX)",
              "    .action(Button::new(\"New window\").variant(ButtonVariant::Outline).icon(icons::PLUS))",
              "EmptyState::new(\"No crates match “blurr”\")",
              "    .description(\"Check the spelling or clear the filter.\")",
              "    .icon(icons::SEARCH_CLOSE)",
              "    .size(EmptyStateSize::Sm)",
              "    .action(Button::new(\"Clear filter\").variant(ButtonVariant::Outline).size(ButtonSize::Sm))",
              "EmptyState::new(\"You're all caught up\")",
              "    .description(\"New review requests will show up here.\")",
              "    .icon(icons::INBOX)",
              "    .size(EmptyStateSize::Lg)",
              "    .action(Button::new(\"Refresh\").variant(ButtonVariant::Secondary).size(ButtonSize::Lg))",
            ],
          ),
        ),
        Section::new(
          "An element in place of the icon",
          "What the app gives stands over the title in place of the well: an illustration, a picture, the people of a \
           project. Two actions stand side by side, the main one last.",
          self.frame.demo(
            1,
            media,
            ".media(element)",
            &[
              "EmptyState::new(\"No one here yet\")",
              "    .description(\"Invite people to work on this project with you.\")",
              "    .media(AvatarGroup::new([",
              "        Avatar::new(\"Ada Lovelace\"),",
              "        Avatar::new(\"Alan Turing\"),",
              "        Avatar::new(\"Grace Hopper\"),",
              "    ]))",
              "    .action(Button::new(\"Not now\").variant(ButtonVariant::Ghost))",
              "    .action(Button::new(\"Invite\").icon(icons::PLUS))",
            ],
          ),
        ),
        Section::new(
          "In the room of a list",
          "Given a height, it is centred in it: here all of the room a panel leaves under its header.",
          self.frame.demo(
            2,
            panel,
            ".height(Length::Fill)",
            &[
              "EmptyState::new(\"No messages\")",
              "    .description(\"Messages you are sent show up here.\")",
              "    .icon(icons::INBOX)",
              "    .size(EmptyStateSize::Sm)",
              "    .height(Length::Fill)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an empty state is given.",
          props_table(&rows, &["title"], self.frame.room()),
        ),
      ],
    )
  }
}
