//! The page of the sidebar: its playground, the board's sidebar beside a page,
//! its rail, and the same sidebar over the page, as on a narrow window.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::{Element, dimension::Dimension},
  ui::{
    Avatar, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Sidebar, SidebarGroup, SidebarHeading,
    SidebarHeadingVariant, SidebarItem, SidebarTrigger, icons,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, segmented_fill, text},
  playground::{Playground, Spec, divider, flag, prop},
};

/// The places of the board's sidebar, by group: a name, an icon, and how many it holds.
const PLACES: [(&str, &str, &str, Option<usize>); 6] = [
  ("Project", "Overview", icons::HOME, None),
  ("Project", "Inbox", icons::INBOX, Some(12)),
  ("Project", "Components", icons::FOLDER, None),
  ("Project", "Tokens", icons::LAYERS, None),
  ("Team", "Members", icons::USERS, None),
  ("Team", "Settings", icons::SETTINGS, None),
];

/// What can be selected: every place, and the nested pages of Components.
const CHOICES: [&str; 8] = [
  "Overview", "Inbox", "Buttons", "Inputs", "Tokens", "Members", "Settings", "None",
];

/// What the playground's sidebar is made of.
#[derive(Clone)]
struct Props {
  collapsed: Signal<bool>,
  open: Signal<bool>,
  narrow: bool,
  groups: bool,
  nested: bool,
  badges: bool,
  collapsible: bool,
  /// Which of [`CHOICES`] is selected, which choosing an item sets.
  selected: Signal<usize>,
}

/// The board's sidebar as `props` say, each item choosing itself and telling `record`.
fn board(props: &Props, record: impl Fn(String) + Clone + Send + Sync + 'static) -> Sidebar {
  let selected = CHOICES[props.selected.get().min(CHOICES.len() - 1)];
  let item = |name: &str, icon: &str| {
    let (choose, record, name_owned) = (props.selected.clone(), record.clone(), name.to_owned());
    SidebarItem::new(name)
      .icon(icon)
      .selected(selected == name)
      .on_select(move || {
        choose.set(CHOICES.iter().position(|choice| *choice == name_owned).unwrap_or(0));
        record(format!("on_select: {name_owned}"));
      })
  };
  let heading = |collapsed: bool| {
    SidebarHeading::new("lurq")
      .description("Workspace")
      .icon(icons::BOX)
      .trailing_icon(icons::CHEVRONS_VERTICAL)
      .collapsed(collapsed)
  };
  let account = |collapsed: bool| {
    SidebarHeading::new("Ivan Zaida")
      .variant(SidebarHeadingVariant::Account)
      .description("ivan@lurq.dev")
      .mark(Avatar::new("Ivan Zaida"))
      .trailing_icon(icons::MORE_HORIZONTAL)
      .collapsed(collapsed)
  };
  let mut groups: Vec<SidebarGroup> = Vec::new();
  for group in ["Project", "Team"] {
    let label = if props.groups { group } else { "" };
    let places: Vec<_> = PLACES.iter().filter(|place| place.0 == group).collect();
    let mut built = SidebarGroup::new(label)
      .key(group)
      .collapsible(props.groups && props.collapsible);
    if props.groups {
      built = built.count(places.len());
    }
    if props.groups && group == "Project" {
      built = built.action(
        Button::new("Add a page")
          .variant(ButtonVariant::Ghost)
          .size(ButtonSize::Sm)
          .icon(icons::PLUS)
          .icon_only(true),
      );
    }
    for (_, name, icon, count) in places {
      let mut place = item(name, icon);
      if *name == "Components" && props.nested {
        place = SidebarItem::new(name)
          .icon(*icon)
          .child(item("Buttons", icons::FILE))
          .child(item("Inputs", icons::FILE).shortcut("Mod+I"));
      }
      if props.badges {
        if let Some(count) = count {
          place = place.count(*count);
        }
        if *name == "Tokens" {
          place = place.badge(Badge::new("New").variant(BadgeVariant::Secondary));
        }
      }
      built = built.item(place);
    }
    groups.push(built);
  }
  Sidebar::new()
    .label("Workspace")
    .header(heading(false))
    .header_collapsed(heading(true))
    .footer(account(false))
    .footer_collapsed(account(true))
    .groups(groups)
    .collapsed(props.collapsed.clone())
    .open(props.open.clone())
    .narrow(props.narrow)
}

/// How high the playground's window is: what the stage has room for under its bar.
const PLAYED: f32 = 340.0;

/// A window as wide as what it is shown in and `height` high: `sidebar` at its own width beside its page, which
/// takes the rest and has the trigger in its bar.
fn window(sidebar: Element, props: &Props, height: impl Into<Dimension>) -> Element {
  let page = Column::new()
    .flex(1.0)
    .height(FILL)
    .padding(16.0)
    .spacing(12.0)
    .child(
      Row::new()
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(
          SidebarTrigger::new(props.collapsed.clone())
            .open(props.open.clone())
            .narrow(props.narrow),
        )
        .child(text(
          CHOICES[props.selected.get().min(CHOICES.len() - 1)],
          15.0,
          FontWeight::SemiBold,
        )),
    )
    .child(text("What the place shows.", 13.0, FontWeight::Normal).color(PaletteColor::MutedForeground));
  Column::new()
    .width(FILL)
    .height(height)
    .border(line())
    .rounded(8.0)
    .clip()
    .child(Row::new().size(FILL, FILL).child(sidebar).child(page))
    .into()
}

pub struct SidebarPage {
  frame: Frame,
  playground: Playground,
  collapsed: Signal<bool>,
  open: Signal<bool>,
  narrow: Signal<bool>,
  groups: Signal<bool>,
  nested: Signal<bool>,
  badges: Signal<bool>,
  collapsible: Signal<bool>,
  selected: Signal<usize>,
  /// The sections' sidebars: beside the page, the rail, over the page.
  beside: (Signal<bool>, Signal<bool>, Signal<usize>),
  rail: (Signal<bool>, Signal<bool>, Signal<usize>),
  over: (Signal<bool>, Signal<bool>, Signal<usize>),
}

impl SidebarPage {
  fn props(&self) -> Props {
    Props {
      collapsed: self.collapsed.clone(),
      open: self.open.clone(),
      narrow: self.narrow.get(),
      groups: self.groups.get(),
      nested: self.nested.get(),
      badges: self.badges.get(),
      collapsible: self.collapsible.get(),
      selected: self.selected.clone(),
    }
  }

  /// The props of a section's sidebar: the board's, collapsed or over the page as the section shows it.
  fn section(signals: &(Signal<bool>, Signal<bool>, Signal<usize>), narrow: bool) -> Props {
    Props {
      collapsed: signals.0.clone(),
      open: signals.1.clone(),
      narrow,
      groups: true,
      nested: true,
      badges: true,
      collapsible: false,
      selected: signals.2.clone(),
    }
  }

  /// The playground: the sidebar in a window, with the trigger in the page's bar.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let record = self.playground.recorder();
    let shown = window(board(&props, record).mount(ctx), &props, PLAYED);

    let mut code = vec![
      "// collapsed, open: Signal<bool>".to_owned(),
      "Sidebar::new()".to_owned(),
      "    .label(\"Workspace\")".to_owned(),
      "    .header(heading(false))".to_owned(),
      "    .header_collapsed(heading(true))".to_owned(),
    ];
    if props.groups {
      let collapsible = if props.collapsible { ".collapsible(true)" } else { "" };
      code.push(format!(
        "    .group(SidebarGroup::new(\"Project\").count(4){collapsible}.action(add).items(project))"
      ));
      code.push(format!(
        "    .group(SidebarGroup::new(\"Team\").count(2){collapsible}.items(team))"
      ));
    } else {
      code
        .push("    .groups([SidebarGroup::new(\"\").items(project), SidebarGroup::new(\"\").items(team)])".to_owned());
    }
    code.push("    .footer(account(false))".to_owned());
    code.push("    .collapsed(collapsed.clone())".to_owned());
    code.push("    .open(open.clone())".to_owned());
    if props.narrow {
      code.push("    .narrow(true)".to_owned());
    }
    code.push("    .mount(ctx)".to_owned());
    let mut item = "SidebarItem::new(\"Inbox\").icon(icons::INBOX)".to_owned();
    if props.badges {
      item.push_str(".count(12)");
    }
    code.push(format!("// {item}.selected(at_inbox)"));
    if props.nested {
      code.push("// SidebarItem::new(\"Components\").child(buttons).child(inputs)".to_owned());
    }

    let tree = vec![
      "root          260 ↔ 52 wide · muted · line at its end".to_owned(),
      "├─ header     heading: mark · title · description".to_owned(),
      "├─ body       scrolls · one list to the keys".to_owned(),
      "│  └─ group   label · count · action".to_owned(),
      "│     └─ item icon · label · count · badge · keys".to_owned(),
      "│        └─ nested   opens with its height".to_owned(),
      "└─ footer     line over it · heading".to_owned(),
    ];

    let choices: Vec<(&str, usize)> = CHOICES.iter().enumerate().map(|(index, name)| (*name, index)).collect();
    let to_selected = self.selected.clone();
    let mut controls = vec![flag("collapsed", &self.collapsed), flag("narrow", &self.narrow)];
    if props.narrow {
      controls.push(flag("open", &self.open));
    }
    controls.push(divider());
    controls.push(flag("groups", &self.groups));
    if props.groups {
      controls.push(flag("collapsible", &self.collapsible));
    }
    controls.push(flag("nested items", &self.nested));
    controls.push(flag("counts and badges", &self.badges));
    controls.push(divider());
    controls.push(prop(
      "selected",
      "bool",
      segmented_fill(&choices[..4], &self.selected.get(), {
        let to = to_selected.clone();
        move |index| to.set(index)
      }),
    ));
    controls.push(segmented_fill(&choices[4..], &self.selected.get(), move |index| to_selected.set(index)).into());

    let resets = (
      [
        self.collapsed.clone(),
        self.narrow.clone(),
        self.open.clone(),
        self.collapsible.clone(),
      ],
      [self.groups.clone(), self.nested.clone(), self.badges.clone()],
      self.selected.clone(),
    );
    let make_props = props.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Sidebar",
        make: Arc::new(move || window(board(&make_props, |_| {}).into(), &make_props, FILL)),
        shown,
        fill: true,
        forceable: false,
        facts: format!(
          "{} · {}",
          if props.narrow {
            "over the page"
          } else if props.collapsed.get() {
            "the rail"
          } else {
            "beside the page"
          },
          CHOICES[props.selected.get().min(CHOICES.len() - 1)]
        ),
        event: "on_select",
        code,
        tree,
        controls,
        changed: [
          props.collapsed.get(),
          props.narrow,
          !props.groups,
          !props.nested,
          !props.badges,
          props.collapsible,
          props.selected.get() != 1,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          for flag in &resets.0 {
            flag.set(false);
          }
          for flag in &resets.1 {
            flag.set(true);
          }
          resets.2.set(1);
        }),
      },
    )
  }
}

impl Component for SidebarPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let mut section = |collapsed: bool| (ctx.signal(collapsed), ctx.signal(false), ctx.signal(1_usize));
    let (beside, rail, over) = (section(false), section(true), section(false));
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      collapsed: ctx.signal(false),
      open: ctx.signal(false),
      narrow: ctx.signal(false),
      groups: ctx.signal(true),
      nested: ctx.signal(true),
      badges: ctx.signal(true),
      collapsible: ctx.signal(false),
      selected: ctx.signal(1),
      beside,
      rail,
      over,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let section = |ctx: &mut Ctx, signals: &(Signal<bool>, Signal<bool>, Signal<usize>), narrow: bool| {
      let props = Self::section(signals, narrow);
      window(board(&props, |_| {}).mount(ctx), &props, 460.0)
    };
    let beside = section(ctx, &self.beside, false);
    let rail = section(ctx, &self.rail, false);
    let over = section(ctx, &self.over, true);
    let rows = [
      (
        "header",
        "impl Into<Element>",
        "",
        "What stays at its top: a SidebarHeading of the app or the workspace, a search.",
      ),
      (
        "header_collapsed",
        "impl Into<Element>",
        "",
        "What the rail shows in the header's place: the mark alone.",
      ),
      (
        "footer",
        "impl Into<Element>",
        "",
        "What stays at its bottom, with a line over it: who is signed in, the settings.",
      ),
      (
        "footer_collapsed",
        "impl Into<Element>",
        "",
        "What the rail shows in the footer's place: an avatar.",
      ),
      (
        "group",
        "SidebarGroup",
        "",
        "A group under a label in capitals, with a count, an action at its end, and collapsible.",
      ),
      (
        "item",
        "SidebarItem",
        "",
        "An item in a group of no label: icon, count, badge, shortcut, selected, disabled, on_select, href, \
         children.",
      ),
      (
        "collapsed",
        "bool | Signal<bool>",
        "false",
        "The rail of the items' icons, 52 wide; its width moves between the two.",
      ),
      (
        "open",
        "Signal<bool>",
        "its own",
        "On a narrow window, whether it is open over the page. Choosing an item closes it.",
      ),
      (
        "narrow",
        "bool",
        "below md",
        "Whether it lies over the page as a drawer does, whatever the window's width.",
      ),
      (
        "label",
        "&str",
        "\"Sidebar\"",
        "What the navigation region is called to tooling.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Navigation", "Sidebar"],
        title: "Sidebar",
        description: "The way round an app, down one side of its window: its groups of places, the one the app is \
                      at, a header with the app or the workspace and a footer with who is signed in.",
        import: Some("use lurq::ui::{Sidebar, SidebarGroup, SidebarItem};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Sidebar is a control on the right. The button in the page's bar collapses it, or opens it \
           over the page where it is narrow. Its items are a tree to the keyboard: Tab stops at the selected \
           one, Up and Down move, Right opens a nested list and goes into it, Left closes it and goes out to the \
           item it is in, and a letter goes to the item it starts.",
          playground,
        ),
        Section::new(
          "Beside the page",
          "260 wide on the muted fill, its header and footer staying while its items scroll. The selected item is \
           on the background with an edge; a nested list opens with its height.",
          self.frame.demo(
            0,
            beside,
            "Sidebar::new()",
            &[
              "Sidebar::new()",
              "    .header(SidebarHeading::new(\"lurq\").description(\"Workspace\").icon(icons::BOX))",
              "    .group(SidebarGroup::new(\"Project\").count(4).items(project))",
              "    .group(SidebarGroup::new(\"Team\").count(2).items(team))",
              "    .footer(account)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "The rail",
          "Collapsed, it is 52 wide and shows the items' icons: each says its name in a tooltip beside it, and an \
           item with nested items opens them in a menu there.",
          self.frame.demo(
            1,
            rail,
            ".collapsed(collapsed.clone())",
            &[
              "// collapsed: Signal<bool>, which SidebarTrigger turns",
              "Sidebar::new()",
              "    .header_collapsed(SidebarHeading::new(\"lurq\").icon(icons::BOX).collapsed(true))",
              "    .groups(groups)",
              "    .collapsed(collapsed.clone())",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Over the page",
          "On a window narrower than the md breakpoint it lies over the page as a drawer does: the trigger opens \
           it, and choosing an item, Escape and a press on the veil close it.",
          self.frame.demo(
            2,
            over,
            ".open(open.clone())",
            &[
              "// open: Signal<bool>, which SidebarTrigger sets on a narrow window",
              "Sidebar::new()",
              "    .groups(groups)",
              "    .open(open.clone())",
              "    .narrow(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a sidebar is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
