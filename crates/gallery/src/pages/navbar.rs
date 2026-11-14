//! The page of the navbar: its playground, a site's header with a search and an avatar with its menu, a link that
//! opens a panel, and the narrow form whose links are behind the menu button.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{self, Column, Row, ScrollVertical},
  core::{ElementRef, Signal},
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{Avatar, Menu, MenuItem, NavItem, NavLink, NavPanel, Navbar, NavbarBrand, SearchField, TextBoxSize, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, segmented_fill, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The bar's links, in order.
const LINKS: [&str; 3] = ["Docs", "Components", "Blog"];

/// How high a window of a navbar is.
const WINDOW: f32 = 340.0;

/// What a navbar of this page is made of.
#[derive(Clone)]
struct Props {
  narrow: bool,
  sticky: bool,
  brand: bool,
  panel: bool,
  search: bool,
  avatar: bool,
  /// Which of [`LINKS`] is the page shown, which choosing a link sets.
  current: Signal<usize>,
  /// What the search holds.
  query: Signal<String>,
  /// The avatar's menu: its anchor and whether it is open.
  account: (ElementRef, Signal<bool>),
  /// The links over the page, on a narrow window.
  menu_open: Signal<bool>,
}

/// The avatar of who is signed in, a button that opens their menu.
fn account(props: &Props, record: impl Fn(String) + Clone + Send + Sync + 'static) -> Element {
  let (anchor, open) = props.account.clone();
  let toggle = open.clone();
  let ring = lurq::ui::current_theme().focus_ring().outline();
  let told = |name: &'static str| {
    let record = record.clone();
    move || record(format!("on_select({name:?})"))
  };
  let menu = Menu::new(open, anchor.clone())
    .item(MenuItem::new("Profile").icon(icons::USER).on_select(told("Profile")))
    .item(
      MenuItem::new("Settings")
        .icon(icons::SETTINGS)
        .on_select(told("Settings")),
    )
    .separator()
    .item(
      MenuItem::new("Sign out")
        .icon(icons::SIGN_OUT)
        .on_select(told("Sign out")),
    );
  Row::new()
    .overflow_visible()
    .child(
      components::Button::empty()
        .tab_index(0)
        .rounded(999.0)
        .focus_visible(|style| style.outline(ring))
        .ref_element(anchor)
        .describe("a11y_name", "Account")
        .describe("a11y_haspopup", "menu")
        .on_click(move |_| toggle.update(|open| *open = !*open))
        .child(Avatar::new("Ivan Zaida")),
    )
    .child(menu)
    .into()
}

/// The navbar as `props` say, each link telling `record` it was chosen.
fn navbar(props: &Props, record: impl Fn(String) + Clone + Send + Sync + 'static) -> Navbar {
  let current = props.current.get();
  let mut bar = Navbar::new()
    .label("Site")
    .narrow(props.narrow)
    .sticky(props.sticky)
    .menu_open(props.menu_open.clone());
  if props.brand {
    let record = record.clone();
    bar = bar.brand(
      NavbarBrand::new("lurq")
        .icon(icons::BOX)
        .on_select(move || record("brand: on_select".to_owned())),
    );
  }
  for (index, name) in LINKS.iter().enumerate() {
    let (choose, record) = (props.current.clone(), record.clone());
    let chosen = move || {
      choose.set(index);
      record(format!("on_select({name:?})"));
    };
    let mut item = NavItem::new(name).current(current == index);
    if index == 0 && props.panel {
      let (start, theming) = (chosen.clone(), chosen.clone());
      item = item.panel(
        NavPanel::new()
          .columns(2)
          .link(
            NavLink::new("Getting started")
              .description("Install lurq and open your first window.")
              .on_select(start),
          )
          .link(
            NavLink::new("Theming")
              .description("Roles, palettes and dark mode.")
              .on_select(theming),
          ),
      );
    } else {
      item = item.on_select(chosen);
    }
    bar = bar.item(item);
  }
  if props.search {
    bar = bar.trailing(
      Row::new().width(200.0).child(
        SearchField::new(props.query.clone())
          .placeholder("Search")
          .size(TextBoxSize::Sm)
          .full_width(true),
      ),
    );
  }
  if props.avatar {
    bar = bar.trailing(account(props, record.clone()));
  }
  let told = record.clone();
  bar
    .on_open_change(move |open| record(format!("on_open_change({open:?})")))
    .on_menu_open_change(move |open| told(format!("on_menu_open_change({open})")))
}

/// A window as wide as what it is shown in and `height` high: the navbar over its page, which scrolls under it.
fn window(navbar: Element, props: &Props, height: f32) -> Element {
  let mut page = Column::new()
    .width(FILL)
    .padding(24.0)
    .spacing(12.0)
    .child(text(
      LINKS[props.current.get().min(LINKS.len() - 1)],
      20.0,
      FontWeight::SemiBold,
    ))
    .child(
      text(
        "What the page shows, under the bar. Scroll it: a sticky bar stays at the top.",
        14.0,
        FontWeight::Normal,
      )
      .color(PaletteColor::MutedForeground),
    );
  for _ in 0..6 {
    page = page.child(
      Column::new()
        .width(FILL)
        .height(72.0)
        .rounded(8.0)
        .background(PaletteColor::Muted),
    );
  }
  Column::new()
    .width(FILL)
    .height(height)
    .border(line())
    .rounded(8.0)
    .clip()
    .background(PaletteColor::Background)
    .child(ScrollVertical::new(Column::new().width(FILL).child(navbar).child(page)).size(FILL, FILL))
    .into()
}

pub struct NavbarPage {
  frame: Frame,
  playground: Playground,
  narrow: Signal<bool>,
  sticky: Signal<bool>,
  brand: Signal<bool>,
  panel: Signal<bool>,
  search: Signal<bool>,
  avatar: Signal<bool>,
  current: Signal<usize>,
  query: Signal<String>,
  account: (ElementRef, Signal<bool>),
  menu_open: Signal<bool>,
  /// The sections' bars: a site's header, the narrow form.
  site: Props,
  phone: Props,
}

impl NavbarPage {
  fn props(&self) -> Props {
    Props {
      narrow: self.narrow.get(),
      sticky: self.sticky.get(),
      brand: self.brand.get(),
      panel: self.panel.get(),
      search: self.search.get(),
      avatar: self.avatar.get(),
      current: self.current.clone(),
      query: self.query.clone(),
      account: self.account.clone(),
      menu_open: self.menu_open.clone(),
    }
  }

  /// The playground: the navbar over a page that scrolls.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let record = self.playground.recorder();
    // Built on the stage, in the stage's theme: the search field in the bar types in that theme's color.
    let shown = Shown::built(|ctx| window(navbar(&props, record).mount(ctx), &props, WINDOW));

    let mut code = vec!["Navbar::new()".to_owned(), "    .label(\"Site\")".to_owned()];
    if props.brand {
      code.push("    .brand(NavbarBrand::new(\"lurq\").icon(icons::BOX).on_select(..))".to_owned());
    }
    for (index, name) in LINKS.iter().enumerate() {
      let current = if props.current.get() == index {
        ".current(true)"
      } else {
        ""
      };
      if index == 0 && props.panel {
        code.push(format!(
          "    .item(NavItem::new({name:?}){current}.panel(NavPanel::new().columns(2).link(start).link(theming)))"
        ));
      } else {
        code.push(format!("    .item(NavItem::new({name:?}){current}.on_select(..))"));
      }
    }
    if props.search {
      code.push(
        "    .trailing(Row::new().width(200.0).child(SearchField::new(query).placeholder(\"Search\").size(TextBoxSize::Sm).full_width(true)))"
          .to_owned(),
      );
    }
    if props.avatar {
      code.push("    .trailing(account) // an Avatar that opens a Menu".to_owned());
    }
    if props.sticky {
      code.push("    .sticky(true)".to_owned());
    }
    if props.narrow {
      code.push("    .narrow(true)".to_owned());
    }
    code.push("    .menu_open(menu_open.clone())".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let tree = vec![
      format!(
        "root        {} · 56 tall · padding 16 · gap 24 · line under it",
        self.playground.measured()
      ),
      "├─ brand    mark 20 · name 16/600".to_owned(),
      "├─ links    NavigationMenu, links variant · 20 apart · 13/500".to_owned(),
      "└─ trailing search · avatar · 24 apart".to_owned(),
      "narrow: menu button · brand · trailing, the links in a Sidebar over the page".to_owned(),
    ];

    let choices: Vec<(&str, usize)> = LINKS.iter().enumerate().map(|(index, name)| (*name, index)).collect();
    let to_current = self.current.clone();
    let controls = vec![
      flag("narrow", &self.narrow),
      flag("sticky", &self.sticky),
      divider(),
      flag("brand", &self.brand),
      flag("Docs opens a panel", &self.panel),
      flag("search", &self.search),
      flag("avatar", &self.avatar),
      divider(),
      prop(
        "current",
        "bool",
        segmented_fill(&choices, &props.current.get(), move |index| to_current.set(index)),
      ),
    ];
    let resets = (
      [self.narrow.clone(), self.sticky.clone(), self.search.clone()],
      [self.brand.clone(), self.panel.clone(), self.avatar.clone()],
      self.current.clone(),
    );
    let make_props = Props {
      account: (ElementRef::new(), Signal::new(false)),
      menu_open: Signal::new(false),
      ..props.clone()
    };
    let make_open = Signal::new(None);
    self.playground.card(
      ctx,
      Spec {
        title: "Navbar",
        make: Arc::new(move || {
          window(
            navbar(&make_props, |_| {}).open(make_open.clone()).into(),
            &make_props,
            WINDOW,
          )
        }),
        shown,
        fill: true,
        forceable: false,
        facts: format!(
          "{} · {}",
          if props.narrow { "narrow" } else { "wide" },
          LINKS[props.current.get().min(LINKS.len() - 1)]
        ),
        event: "on_select",
        code,
        tree,
        controls,
        changed: [
          props.narrow,
          props.sticky,
          !props.brand,
          !props.panel,
          props.search,
          !props.avatar,
          props.current.get() != 0,
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
          resets.2.set(0);
        }),
      },
    )
  }
}

impl Component for NavbarPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let mut section = |narrow: bool| Props {
      narrow,
      sticky: false,
      brand: true,
      panel: true,
      search: !narrow,
      avatar: true,
      current: ctx.signal(0),
      query: ctx.signal(String::new()),
      account: (ElementRef::new(), ctx.signal(false)),
      menu_open: ctx.signal(false),
    };
    let (site, phone) = (section(false), section(true));
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      narrow: ctx.signal(false),
      sticky: ctx.signal(false),
      brand: ctx.signal(true),
      panel: ctx.signal(true),
      // Off at first: the board's bar with its search of 200 is wider than the stage. The section of a site's
      // header has it.
      search: ctx.signal(false),
      avatar: ctx.signal(true),
      current: ctx.signal(0),
      query: ctx.signal(String::new()),
      account: (ElementRef::new(), ctx.signal(false)),
      menu_open: ctx.signal(false),
      site,
      phone,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let site = window(navbar(&self.site, |_| {}).mount(ctx), &self.site, WINDOW);
    let phone = Row::new()
      .width(FILL)
      .justify(lurq::layout::layout_kind::Justify::Center)
      .child(Column::new().width(380.0).align_items(Alignment::Stretch).child(window(
        navbar(&self.phone, |_| {}).mount(ctx),
        &self.phone,
        WINDOW,
      )));

    let rows = [
      (
        "brand",
        "impl Into<Element>",
        "",
        "What stands at its start: a NavbarBrand (an icon or its media beside a name, pressed where it has \
         on_select), or an element of the app's.",
      ),
      (
        "item",
        "NavItem",
        "",
        "A link after the others: href or on_select, current, icon, disabled, and panel(NavPanel) for one that \
         opens a panel of links.",
      ),
      (
        "trailing",
        "impl Into<Element>",
        "",
        "What stands after the links, after what stands there: a SearchField, a Button, an Avatar with its Menu.",
      ),
      (
        "sticky",
        "bool",
        "false",
        "It stays at the top of the page that scrolls it.",
      ),
      (
        "open",
        "Signal<Option<usize>>",
        "its own",
        "Which item's panel is open, by its index: the app's.",
      ),
      (
        "on_open_change",
        "Fn(Option<usize>)",
        "",
        "What hears a panel open, another take its place, or the open one close.",
      ),
      (
        "menu_open",
        "Signal<bool>",
        "its own",
        "On a narrow window, whether the links are open over the page. Choosing one closes them.",
      ),
      (
        "on_menu_open_change",
        "Fn(bool)",
        "",
        "What hears the menu button open the links over the page, or close them.",
      ),
      (
        "narrow",
        "bool",
        "below md",
        "Whether its links are behind the menu button, whatever the window's width.",
      ),
      (
        "label",
        "&str",
        "\"Main\"",
        "What the navigation region is called to tooling.",
      ),
      (
        "menu_label",
        "&str",
        "\"Menu\"",
        "What the menu button of the narrow form is called to tooling: whether the links are open it says on its own.",
      ),
      (
        "id",
        "impl Into<Arc<str>>",
        "",
        "The bar's id; its links are {id}-links-0 and on, the menu button {id}-menu, the links over the page \
         {id}-drawer.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Navigation", "Navbar"],
        title: "Navbar",
        description: "The way round a site or an app along the top of its window: the brand at its start, its \
                      links, and a search, actions and who is signed in at its end.",
        import: Some("use lurq::ui::{NavItem, Navbar, NavbarBrand};"),
        status: Some("Beta"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Navbar is a control on the right. Its links are a navigation menu: each a stop of Tab, \
           the page shown in the foreground; Docs opens its panel by a press, under the pointer after a moment, \
           and with Enter, Space and Down. Narrow, the links go behind the menu button into a sidebar over the page.",
          playground,
        ),
        Section::new(
          "A site's header",
          "The board's: the brand, Docs with its panel, Components and Blog, a search of 200 and the avatar of who \
           is signed in, which opens their menu.",
          self.frame.demo(
            0,
            site,
            "Navbar::new().brand(..).item(..).trailing(..)",
            &[
              "// menu_open: Signal<bool>; query: Signal<String>; account: an Avatar that opens a Menu",
              "Navbar::new()",
              "    .label(\"Site\")",
              "    .brand(NavbarBrand::new(\"lurq\").icon(icons::BOX).on_select(..))",
              "    .item(NavItem::new(\"Docs\").current(true).panel(",
              "        NavPanel::new()",
              "            .columns(2)",
              "            .link(NavLink::new(\"Getting started\").description(\"Install lurq and open your first window.\"))",
              "            .link(NavLink::new(\"Theming\").description(\"Roles, palettes and dark mode.\")),",
              "    ))",
              "    .item(NavItem::new(\"Components\").on_select(..))",
              "    .item(NavItem::new(\"Blog\").on_select(..))",
              "    .trailing(Row::new().width(200.0).child(SearchField::new(query).placeholder(\"Search\").size(TextBoxSize::Sm).full_width(true)))",
              "    .trailing(account)",
              "    .menu_open(menu_open.clone())",
              "    .narrow(false)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "On a narrow window",
          "Below the md breakpoint the links go behind the menu button at the bar's start, into a sidebar over the \
           page: an item with a panel holds its links there, and choosing one closes it.",
          self.frame.demo(
            1,
            phone,
            ".narrow(true)",
            &[
              "// menu_open: Signal<bool>, which the menu button turns",
              "Navbar::new()",
              "    .label(\"Site\")",
              "    .brand(NavbarBrand::new(\"lurq\").icon(icons::BOX).on_select(..))",
              "    .item(NavItem::new(\"Docs\").current(true).panel(docs))",
              "    .item(NavItem::new(\"Components\").on_select(..))",
              "    .item(NavItem::new(\"Blog\").on_select(..))",
              "    .trailing(account)",
              "    .menu_open(menu_open.clone())",
              "    .narrow(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a navbar is given. Its items are a navigation menu's, with all a NavItem has.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
