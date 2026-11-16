//! The page of the navigation menu: its playground, a site's menu whose panels are columns of links, a panel with
//! icons and a featured card, and the links variant a navbar has.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Card, CardVariant, NavItem, NavLink, NavPanel, NavigationMenu, NavigationMenuVariant, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, fit, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The links of the Docs panel: a title, a line about it, and an icon.
const DOCS: [(&str, &str, &str); 4] = [
  (
    "Getting started",
    "Install lurq and open your first window.",
    icons::PLAY_CIRCLE,
  ),
  ("Theming", "Roles, palettes and dark mode.", icons::PALETTE),
  (
    "Layout",
    "Rows, columns, stacks and the room they share.",
    icons::LAYERS,
  ),
  ("Accessibility", "What tooling reads, and the keyboard.", icons::EYE),
];

/// The links of the Components panel.
const COMPONENTS: [(&str, &str, &str); 3] = [
  (
    "Button",
    "A press: the main action, an alternative, a ghost.",
    icons::CURSOR,
  ),
  ("Input", "A line of text, a password, a number.", icons::TEXT_CURSOR),
  ("Menu", "Commands under a button, with shortcuts.", icons::LIST),
];

/// How high the scene of a menu is: room for its bar and the panel of two columns under it.
const SCENE: f32 = 300.0;

/// How high the scene of a panel of one column of four links with icons is.
const TALL: f32 = 400.0;

/// What the playground's menu is made of.
#[derive(Clone, Copy)]
struct Props {
  variant: NavigationMenuVariant,
  columns: usize,
  descriptions: bool,
  icons: bool,
  featured: bool,
  disabled: bool,
}

/// The links `of`, as `props` say, each telling `record` it was chosen.
fn links(
  of: &[(&str, &str, &str)],
  props: Props,
  record: &(impl Fn(String) + Clone + Send + Sync + 'static),
) -> Vec<NavLink> {
  of.iter()
    .map(|(title, description, icon)| {
      let (record, name) = (record.clone(), title.to_string());
      let mut link = NavLink::new(title).on_select(move || record(format!("on_select({name:?})")));
      if props.descriptions {
        link = link.description(description);
      }
      if props.icons {
        link = link.icon(*icon);
      }
      link
    })
    .collect()
}

/// The card the Docs panel features.
fn release() -> Card {
  Card::new()
    .variant(CardVariant::Filled)
    .title("lurq 0.62")
    .description("A navbar, a navigation menu, a sidebar and a tree view.")
}

/// The menu as `props` say: Docs and Components open panels, Blog is a link, Changelog is one that is disabled
/// where they say so.
fn menu(props: Props, record: impl Fn(String) + Clone + Send + Sync + 'static) -> NavigationMenu {
  let mut docs = NavPanel::new()
    .columns(props.columns)
    .links(links(&DOCS, props, &record));
  if props.featured {
    docs = docs.featured(release());
  }
  let blog = {
    let record = record.clone();
    NavItem::new("Blog").on_select(move || record("on_select(\"Blog\")".to_owned()))
  };
  let told = record.clone();
  NavigationMenu::new()
    .label("Site")
    .variant(props.variant)
    .item(NavItem::new("Docs").panel(docs))
    .item(NavItem::new("Components").panel(NavPanel::new().links(links(&COMPONENTS, props, &record))))
    .item(blog)
    .item(NavItem::new("Changelog").disabled(props.disabled))
    .on_open_change(move |open| told(format!("on_open_change({open:?})")))
}

/// How wide the menu's four items are in their row, as a menu and as links: the row neither wraps nor gets
/// narrower.
const LIST: f32 = 340.0;
const LIST_OF_LINKS: f32 = 292.0;

/// A scene as wide as what it is shown in and `height` high, the menu at its top.
fn scene(menu: Element, height: f32) -> Element {
  Column::new()
    .width(FILL)
    .height(height)
    .align_items(Alignment::Start)
    .child(Row::new().overflow_visible().child(menu))
    .into()
}

/// A scene on a stage with `room`: where that is less than the menu's items take, the scene is as wide as they
/// are and scrolls sideways.
fn staged(key: &str, room: f32, variant: NavigationMenuVariant, scene: Element) -> Element {
  let least = match variant {
    NavigationMenuVariant::Links => LIST_OF_LINKS,
    NavigationMenuVariant::Menu => LIST,
  };
  fit(key, room, least, scene)
}

pub struct NavigationMenuPage {
  frame: Frame,
  playground: Playground,
  variant: Signal<NavigationMenuVariant>,
  columns: Signal<usize>,
  descriptions: Signal<bool>,
  icons: Signal<bool>,
  featured: Signal<bool>,
  disabled: Signal<bool>,
  play_open: Signal<Option<usize>>,
  /// The sections' menus: a site's, icons and a card, the links variant.
  site_open: Signal<Option<usize>>,
  rich_open: Signal<Option<usize>>,
  links_open: Signal<Option<usize>>,
}

impl NavigationMenuPage {
  fn props(&self) -> Props {
    Props {
      variant: self.variant.get(),
      columns: self.columns.get(),
      descriptions: self.descriptions.get(),
      icons: self.icons.get(),
      featured: self.featured.get(),
      disabled: self.disabled.get(),
    }
  }

  /// The playground: the menu at the top of the stage, its panels opening over it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let record = self.playground.recorder();
    let height = if props.columns == 1 { TALL } else { SCENE };
    let shown = scene(menu(props, record).open(self.play_open.clone()).mount(ctx), height);
    let shown = staged("played", self.frame.stage_room(), props.variant, shown);

    let mut code = vec!["NavigationMenu::new()".to_owned(), "    .label(\"Site\")".to_owned()];
    if props.variant == NavigationMenuVariant::Links {
      code.push("    .variant(NavigationMenuVariant::Links)".to_owned());
    }
    let link = match (props.descriptions, props.icons) {
      (true, true) => "NavLink::new(title).description(line).icon(icon)",
      (true, false) => "NavLink::new(title).description(line)",
      (false, true) => "NavLink::new(title).icon(icon)",
      (false, false) => "NavLink::new(title)",
    };
    code.push(format!("    // each link: {link}.on_select(..)"));
    let mut docs = format!("NavPanel::new().columns({}).links(docs)", props.columns);
    if props.featured {
      docs.push_str(".featured(release)");
    }
    code.push(format!("    .item(NavItem::new(\"Docs\").panel({docs}))"));
    code.push("    .item(NavItem::new(\"Components\").panel(NavPanel::new().links(components)))".to_owned());
    code.push("    .item(NavItem::new(\"Blog\").on_select(..))".to_owned());
    code.push(format!(
      "    .item(NavItem::new(\"Changelog\"){})",
      if props.disabled { ".disabled(true)" } else { "" }
    ));
    code.push("    .on_open_change(..)".to_owned());
    code.push("    .open(open.clone())".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let tree = vec![
      format!("bar          {} · items 4 apart", self.playground.measured()),
      "├─ item       36 tall · padding 12 · 13/500 · chevron 14".to_owned(),
      "└─ viewport   6 under its item".to_owned(),
      "   └─ panel   popover · radius lg · padding 8 · columns 208".to_owned(),
      "      ├─ featured   a column of its own".to_owned(),
      "      └─ link       padding 12 · icon · title 14/500 · description 13".to_owned(),
    ];
    let facts = match self.play_open.get() {
      Some(index) => format!("open: {index}"),
      None => "closed".to_owned(),
    };

    let (to_variant, to_columns) = (self.variant.clone(), self.columns.clone());
    let variants = [
      ("Menu", NavigationMenuVariant::Menu),
      ("Links", NavigationMenuVariant::Links),
    ];
    let columns = [("1", 1_usize), ("2", 2), ("3", 3)];
    let controls = vec![
      prop(
        "variant",
        "NavigationMenuVariant",
        segmented_fill(&variants, &props.variant, move |variant| to_variant.set(variant)),
      ),
      prop(
        "columns",
        "usize",
        segmented_fill(&columns, &props.columns, move |count| to_columns.set(count)),
      ),
      divider(),
      flag("descriptions", &self.descriptions),
      flag("icons", &self.icons),
      flag("featured", &self.featured),
      flag("Changelog disabled", &self.disabled),
    ];
    let resets = (
      self.variant.clone(),
      self.columns.clone(),
      [self.descriptions.clone(), self.disabled.clone()],
      [self.icons.clone(), self.featured.clone()],
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Navigation Menu",
        make: Arc::new({
          // The window of its own keeps which panel is open in a signal of its own.
          let open = Signal::new(None);
          move || scene(menu(props, |_| {}).open(open.clone()).into(), height)
        }),
        shown: Shown::of(shown),
        fill: true,
        forceable: false,
        facts,
        event: "on_select",
        code,
        tree,
        controls,
        changed: [
          props.variant != NavigationMenuVariant::Menu,
          props.columns != 2,
          !props.descriptions,
          props.icons,
          props.featured,
          !props.disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          resets.0.set(NavigationMenuVariant::Menu);
          resets.1.set(2);
          for flag in &resets.2 {
            flag.set(true);
          }
          for flag in &resets.3 {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for NavigationMenuPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      variant: ctx.signal(NavigationMenuVariant::Menu),
      columns: ctx.signal(2),
      descriptions: ctx.signal(true),
      icons: ctx.signal(false),
      featured: ctx.signal(false),
      disabled: ctx.signal(true),
      play_open: ctx.signal(None),
      site_open: ctx.signal(None),
      rich_open: ctx.signal(None),
      links_open: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let quiet_record = |_: String| {};
    let board = Props {
      variant: NavigationMenuVariant::Menu,
      columns: 2,
      descriptions: true,
      icons: false,
      featured: false,
      disabled: false,
    };
    let room = self.frame.stage_room();
    let site = scene(menu(board, quiet_record).open(self.site_open.clone()).mount(ctx), SCENE);
    let site = staged("site", room, board.variant, site);
    let rich = scene(
      menu(
        Props {
          columns: 1,
          icons: true,
          featured: true,
          ..board
        },
        quiet_record,
      )
      .open(self.rich_open.clone())
      .mount(ctx),
      TALL,
    );
    let rich = staged("rich", room, board.variant, rich);
    let links = scene(
      menu(
        Props {
          variant: NavigationMenuVariant::Links,
          ..board
        },
        quiet_record,
      )
      .open(self.links_open.clone())
      .mount(ctx),
      SCENE,
    );
    let links = staged("links", room, NavigationMenuVariant::Links, links);
    let state = |open: &Signal<Option<usize>>| {
      quiet(
        &match open.get() {
          Some(index) => format!("open: {index}"),
          None => "closed".to_owned(),
        },
        12.0,
      )
    };

    let rows = [
      (
        "item",
        "NavItem",
        "",
        "An item after the others: NavItem::new(label) with href or on_select, current, icon, disabled, id, key, \
         and panel(NavPanel) for one that opens a panel of links.",
      ),
      (
        "NavPanel",
        "links, columns, featured",
        "1 column",
        "What an item opens: its NavLinks in columns, the first filled first, and what the app features in a \
         column of its own before them.",
      ),
      (
        "NavLink",
        "title, description, icon, href, on_select, current, disabled, id, key",
        "",
        "A link of a panel. Choosing it closes the panel, then runs it and goes to its path.",
      ),
      (
        "open",
        "Signal<Option<usize>>",
        "its own",
        "Which item's panel is open, by its index: the app's. A press, the pointer and the keys set it.",
      ),
      (
        "on_open_change",
        "Fn(Option<usize>)",
        "",
        "What hears a panel open, another take its place, or the open one close.",
      ),
      (
        "variant",
        "impl Into<Name>",
        "Menu",
        "Menu: each item a box lit under the pointer. Links: the text alone, as a navbar has it.",
      ),
      (
        "open_delay",
        "impl Into<TransitionDuration>",
        "base, 160 ms",
        "How long the pointer rests on an item before its panel opens.",
      ),
      (
        "close_delay",
        "impl Into<TransitionDuration>",
        "slow, 240 ms",
        "How long a panel stays after the pointer has left it and its item.",
      ),
      ("disabled", "bool", "false", "No item can be pressed."),
      (
        "label",
        "&str",
        "\"Main\"",
        "What the navigation region is called to tooling.",
      ),
      (
        "id",
        "impl Into<Arc<str>>",
        "",
        "The row's id; its items are {id}-0, {id}-1 and on, each panel {id}-0-panel, its links {id}-0-0 and on.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Navigation", "Navigation Menu"],
        title: "Navigation Menu",
        description: "The way round a site along a row of links, some of which open a panel of links under them: \
                      by a press, and under the pointer after a moment.",
        import: Some("use lurq::ui::{NavItem, NavLink, NavPanel, NavigationMenu};"),
        status: Some("Beta"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The props of the menu and of its panels are controls on the right. Rest the pointer on Docs and its \
           panel opens; with one open, the pointer on Components hands it over at once. Each item is a stop of \
           Tab; Enter, Space and Down open a panel with the focus on its first link, Up and Down go through the \
           links, Left and Right go to the item beside it, and Escape closes it with the focus back on its item.",
          playground,
        ),
        Section::new(
          "A site's menu",
          "The board's: Docs opens two columns of links, each a title over a line about where it goes, under the \
           item, which is lit while its panel is open. A press outside, a choice and Escape close it, and so does \
           the pointer leaving both for a moment.",
          self.frame.demo(
            0,
            Column::new()
              .width(FILL)
              .spacing(8.0)
              .child(site)
              .child(state(&self.site_open)),
            "NavigationMenu::new().item(NavItem::new(\"Docs\").panel(..))",
            &[
              "// open: Signal<Option<usize>>",
              "NavigationMenu::new()",
              "    .label(\"Site\")",
              "    .item(NavItem::new(\"Docs\").panel(",
              "        NavPanel::new().columns(2).links([",
              "            NavLink::new(\"Getting started\").description(\"Install lurq and open your first window.\"),",
              "            NavLink::new(\"Theming\").description(\"Roles, palettes and dark mode.\"),",
              "            NavLink::new(\"Layout\").description(\"Rows, columns, stacks and the room they share.\"),",
              "            NavLink::new(\"Accessibility\").description(\"What tooling reads, and the keyboard.\"),",
              "        ]),",
              "    ))",
              "    .item(NavItem::new(\"Components\").panel(NavPanel::new().links(components)))",
              "    .item(NavItem::new(\"Blog\").on_select(..))",
              "    .item(NavItem::new(\"Changelog\"))",
              "    .open(open.clone())",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Icons and a featured card",
          "A link may have an icon at its start, and a panel what the app features in a column of its own before \
           its links: a card of the newest release.",
          self.frame.demo(
            1,
            rich,
            "NavPanel::new().featured(card)",
            &[
              "let release = Card::new()",
              "    .variant(CardVariant::Filled)",
              "    .title(\"lurq 0.62\")",
              "    .description(\"A navbar, a navigation menu, a sidebar and a tree view.\");",
              "NavigationMenu::new()",
              "    .item(NavItem::new(\"Docs\").panel(",
              "        NavPanel::new()",
              "            .featured(release)",
              "            .link(NavLink::new(\"Getting started\").description(..).icon(icons::PLAY_CIRCLE))",
              "            .link(NavLink::new(\"Theming\").description(..).icon(icons::PALETTE))",
              "            .link(NavLink::new(\"Layout\").description(..).icon(icons::LAYERS))",
              "            .link(NavLink::new(\"Accessibility\").description(..).icon(icons::EYE)),",
              "    ))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "The links variant",
          "Each item its text alone, in the muted foreground, brought forward under the pointer, while its panel is \
           open and where it is the page shown: the links of a Navbar.",
          self.frame.demo(
            2,
            links,
            ".variant(NavigationMenuVariant::Links)",
            &[
              "NavigationMenu::new()",
              "    .variant(NavigationMenuVariant::Links)",
              "    .item(NavItem::new(\"Docs\").panel(docs))",
              "    .item(NavItem::new(\"Components\").panel(components))",
              "    .item(NavItem::new(\"Blog\").on_select(..))",
              "    .item(NavItem::new(\"Changelog\"))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a navigation menu, its items and its panels are given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
